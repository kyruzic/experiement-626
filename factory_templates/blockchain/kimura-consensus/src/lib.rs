pub mod election;
pub mod engine;
pub mod validator;

pub use election::Election;
pub use engine::ConsensusEngine;
pub use validator::Validator;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_engine() {
        // TODO: Implement test
    }

    #[test]
    fn test_validator() {
        // TODO: Implement test
    }

    #[test]
    fn test_election() {
        // TODO: Implement test
    }
}
