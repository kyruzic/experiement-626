pub mod engine;
pub mod validator;
pub mod election;

pub use engine::ConsensusEngine;
pub use validator::Validator;
pub use election::Election;

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