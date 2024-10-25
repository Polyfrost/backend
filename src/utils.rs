macro_rules! check_internal_error {
	( $e:expr ) => {
		match $e {
			Ok(x) => x,
			Err(e) => {
				return actix_web::HttpResponse::InternalServerError().json(structs::ErrorResponse {
					error: "INTERNAL_SERVER_ERROR".to_string(),
					message: e.to_string(),
				})
			}
		}
	};
	( $e:expr, $err:expr ) => {
		match $e {
			Some(x) => x,
			None => {
				return HttpResponse::InternalServerError().json(structs::ErrorResponse {
					error: "INTERNAL_SERVER_ERROR".to_string(),
					message: $err.to_string(),
				})
			}
		}
	};
}
