mod simple;

use cozy_chess::Board;

pub use simple::SimpleEvaluator;

/// Position evaluation score in centipawns (positive favors White, negative favors Black).
pub type Score = i32;

/// Positive infinity score (e.g. Black is checkmated).
pub const INF: Score = Score::MAX;

/// Negative infinity score (e.g. White is checkmated), symmetric with [`INF`].
pub const NEG_INF: Score = -INF;

/// Abstraction for a chess position evaluation function.
///
/// Implementations take a [`Board`] and return a numeric [`Score`].
pub trait Evaluator {
    fn evaluate(&self, board: &Board) -> Score;
}

impl<F> Evaluator for F
where
    F: Fn(&Board) -> Score,
{
    fn evaluate(&self, board: &Board) -> Score {
        self(board)
    }
}

/// Placeholder evaluator that always returns `0`.
#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy)]
pub struct ZeroEvaluator;

impl Evaluator for ZeroEvaluator {
    fn evaluate(&self, _board: &Board) -> Score {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct PieceCountEvaluator;

    impl Evaluator for PieceCountEvaluator {
        fn evaluate(&self, board: &Board) -> Score {
            board.occupied().len() as Score
        }
    }

    fn eval_fn(board: &Board) -> Score {
        board.colors(board.side_to_move()).len() as Score
    }

    #[test]
    fn zero_evaluator_returns_zero() {
        let board = Board::default();
        assert_eq!(ZeroEvaluator.evaluate(&board), 0);
    }

    #[test]
    fn custom_struct_implements_evaluator() {
        let board = Board::default();
        assert_eq!(PieceCountEvaluator.evaluate(&board), 32);
    }

    #[test]
    fn function_and_closure_implement_evaluator() {
        let board = Board::default();
        let fn_eval: fn(&Board) -> Score = eval_fn;
        assert_eq!(fn_eval.evaluate(&board), 16);

        let closure_eval = |b: &Board| b.checkers().len() as Score;
        assert_eq!(closure_eval.evaluate(&board), 0);
    }

    #[test]
    fn evaluator_is_object_safe() {
        let board = Board::default();
        let boxed: Box<dyn Evaluator> = Box::new(PieceCountEvaluator);
        assert_eq!(boxed.evaluate(&board), 32);
    }
}
