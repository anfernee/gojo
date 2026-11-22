use cozy_chess::{Board, Color, Piece};

use crate::eval::{Evaluator, INF, NEG_INF, Score};

const QUEEN_VALUE: Score = 900;
const ROOK_VALUE: Score = 500;
const BISHOP_VALUE: Score = 310;
const KNIGHT_VALUE: Score = 300;
const PAWN_VALUE: Score = 100;

/// Simple material and checkmate evaluator based on `src/eval/rule.md`.
///
/// Scores are in centipawns with White positive and Black negative:
/// - Black checkmated: `INF`
/// - White checkmated: `NEG_INF` (`-INF`)
/// - Queen = 900 (9), Rook = 500 (5), Bishop = 310 (3.1), Knight = 300 (3), Pawn = 100 (1)
#[derive(Debug, Default, Clone, Copy)]
pub struct SimpleEvaluator;

impl Evaluator for SimpleEvaluator {
    fn evaluate(&self, board: &Board) -> Score {
        if !board.checkers().is_empty() && !board.generate_moves(|_| true) {
            return match board.side_to_move() {
                Color::Black => INF,
                Color::White => NEG_INF,
            };
        }

        const PIECES: [(Piece, Score); 5] = [
            (Piece::Queen, QUEEN_VALUE),
            (Piece::Rook, ROOK_VALUE),
            (Piece::Bishop, BISHOP_VALUE),
            (Piece::Knight, KNIGHT_VALUE),
            (Piece::Pawn, PAWN_VALUE),
        ];

        let mut score: Score = 0;
        for (piece, value) in PIECES {
            let white_count = board.colored_pieces(Color::White, piece).len() as Score;
            let black_count = board.colored_pieces(Color::Black, piece).len() as Score;
            score += (white_count - black_count) * value;
        }

        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_fen(fen: &str) -> Score {
        let board: Board = fen.parse().expect("valid FEN");
        SimpleEvaluator.evaluate(&board)
    }

    #[test]
    fn startpos_is_equal() {
        assert_eq!(SimpleEvaluator.evaluate(&Board::default()), 0);
    }

    #[test]
    fn black_checkmated_is_positive_infinity() {
        // Scholar's mate: Black to move and checkmated.
        let mut board = Board::default();
        for mv in ["e2e4", "e7e5", "f1c4", "b8c6", "d1h5", "g8f6", "h5f7"] {
            board.play(mv.parse().unwrap());
        }
        assert_eq!(SimpleEvaluator.evaluate(&board), INF);
    }

    #[test]
    fn white_checkmated_is_negative_infinity() {
        // Fool's mate: White to move and checkmated.
        let mut board = Board::default();
        for mv in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            board.play(mv.parse().unwrap());
        }
        assert_eq!(SimpleEvaluator.evaluate(&board), NEG_INF);
    }

    #[test]
    fn checkmate_overrides_material_imbalance() {
        // White has only a rook and king vs Black's queen and rook, yet Black is checkmated on a8.
        assert_eq!(eval_fen("k2R4/8/1K6/8/8/8/8/5qr1 b - - 0 1"), INF);
    }

    #[test]
    fn individual_piece_values() {
        // White pieces (positive)
        assert_eq!(eval_fen("4k3/8/8/8/8/8/8/3QK3 w - - 0 1"), 900);
        assert_eq!(eval_fen("4k3/8/8/8/8/8/8/3RK3 w - - 0 1"), 500);
        assert_eq!(eval_fen("4k3/8/8/8/8/8/8/3BK3 w - - 0 1"), 310);
        assert_eq!(eval_fen("4k3/8/8/8/8/8/8/3NK3 w - - 0 1"), 300);
        assert_eq!(eval_fen("4k3/8/8/8/8/8/4P3/4K3 w - - 0 1"), 100);

        // Black pieces (negative)
        assert_eq!(eval_fen("3qk3/8/8/8/8/8/8/4K3 w - - 0 1"), -900);
        assert_eq!(eval_fen("3rk3/8/8/8/8/8/8/4K3 w - - 0 1"), -500);
        assert_eq!(eval_fen("3bk3/8/8/8/8/8/8/4K3 w - - 0 1"), -310);
        assert_eq!(eval_fen("3nk3/8/8/8/8/8/8/4K3 w - - 0 1"), -300);
        assert_eq!(eval_fen("4k3/4p3/8/8/8/8/8/4K3 w - - 0 1"), -100);
    }

    #[test]
    fn combined_material_and_turn_invariance() {
        // White: Q (900) + B (310) + 2P (200) = 1410
        // Black: R (500) + N (300) + P (100) = 900
        // Net: +510 regardless of side to move.
        let white_to_move = "3rk2n/4p3/8/8/8/8/3PP3/2BQK3 w - - 0 1";
        let black_to_move = "3rk2n/4p3/8/8/8/8/3PP3/2BQK3 b - - 0 1";
        assert_eq!(eval_fen(white_to_move), 510);
        assert_eq!(eval_fen(black_to_move), 510);
    }
}
