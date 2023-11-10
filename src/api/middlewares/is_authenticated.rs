use std::future::{ready, Ready};

use actix_identity::IdentityExt;
use actix_web::{
    body::EitherBody,
    dev::{self, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;

use crate::app_error::{self};

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct IsAuthenticated;

// Middleware factory is `Transform` trait from actix-service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for IsAuthenticated
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = IsAuthenticatedMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(IsAuthenticatedMiddleware { service }))
    }
}

pub struct IsAuthenticatedMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for IsAuthenticatedMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    dev::forward_ready!(service);

    fn call(&self, s_req: ServiceRequest) -> Self::Future {
        let identity = s_req.get_identity();

        match identity {
            Ok(identity) => match identity.id() {
                Ok(_uid) => {
                    let res = self.service.call(s_req);
                    Box::pin(async move { res.await.map(ServiceResponse::map_into_left_body) })
                }
                Err(_) => unauthorized_response(&s_req),
            },
            Err(_) => unauthorized_response(&s_req),
        }
    }
}

fn unauthorized_response<B>(
    s_req: &ServiceRequest,
) -> LocalBoxFuture<'static, Result<ServiceResponse<EitherBody<B>>, Error>>
where
    B: 'static,
{
    let request = s_req.request().clone();
    let response = HttpResponse::Unauthorized()
        .json(app_error::ErrorResponse::new(
            401,
            "You are not authenticated".into(),
        ))
        .map_into_right_body::<B>();

    Box::pin(async { Ok(ServiceResponse::new(request, response)) })
}