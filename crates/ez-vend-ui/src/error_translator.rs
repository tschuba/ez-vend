use crate::i18n::translate_with_params;
use domain::{DomainError, ValidationError};
use std::collections::HashMap;

pub fn translate_validation_error(error: &ValidationError) -> String {
    translate_with_params(error.key(), HashMap::from_iter(error.params()))
}

pub fn translate_domain_error(error: &DomainError) -> String {
    match error {
        DomainError::Validation(validation) => translate_validation_error(validation),
        DomainError::NotFound(_) => translate_with_params("error.not_found", HashMap::new()),
        DomainError::Storage(_) => translate_with_params("error.storage", HashMap::new()),
        DomainError::InvalidState(_) => translate_with_params("error.generic", HashMap::new()),
    }
}
