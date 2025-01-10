use validator::Validate;

pub fn validate_payload<T: Validate>(payload: &T) -> Result<(), actix_web::Error> {
    payload.validate()
        .map_err(|err| actix_web::error::ErrorBadRequest(err))
}