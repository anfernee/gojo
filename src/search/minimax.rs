use cozy_chess::{Board, Color, Move};

use crate::eval::{Evaluator, Score};
use crate::search::{SearchConfig, SearchResult, Searcher, StopSignal};

/// Classic minimax search algorithm.
///
/// White maximizes the evaluation score and Black minimizes it, matching the
/// White-positive / Black-negative score convention of [`Evaluator`].
#[derive(Debug, Default, Clone, Copy)]
pub struct MinimaxSearcher;

impl MinimaxSearcher {
    pub fn new() -> Self {
        Self
    }

    fn minimax(
        board: &Board,
        depth: u32,
        evaluator: &dyn Evaluator,
        stop: &StopSignal,
        nodes: &mut u64,
    ) -> Option<Score> {
        if stop.is_stopped() {
            return None;
        }

        *nodes += 1;

        if depth == 0 {
            return Some(evaluator.evaluate(board));
        }

        let mut moves = Vec::new();
        board.generate_moves(|piece_moves| {
            moves.extend(piece_moves);
            false
        });

        if moves.is_empty() {
            return if board.checkers().is_empty() {
                Some(0)
            } else {
                Some(evaluator.evaluate(board))
            };
        }

        let maximizing = board.side_to_move() == Color::White;
        let mut best_score: Option<Score> = None;

        for mv in moves {
            let mut child = board.clone();
            child.play_unchecked(mv);
            let score = Self::minimax(&child, depth - 1, evaluator, stop, nodes)?;

            best_score = Some(match best_score {
                None => score,
                Some(cur) if maximizing => cur.max(score),
                Some(cur) => cur.min(score),
            });
        }

        best_score
    }
}

impl Searcher for MinimaxSearcher {
    fn search(
        &mut self,
        board: &Board,
        evaluator: &dyn Evaluator,
        config: &SearchConfig,
        stop: &StopSignal,
    ) -> SearchResult {
        if stop.is_stopped() {
            return SearchResult {
                best_move: None,
                score: 0,
                depth: config.depth,
                nodes: 0,
                stopped: true,
            };
        }

        let mut nodes = 1u64;

        if config.depth == 0 {
            return SearchResult {
                best_move: None,
                score: evaluator.evaluate(board),
                depth: 0,
                nodes,
                stopped: false,
            };
        }

        let root_moves: Vec<Move> = if !config.searchmoves.is_empty() {
            config
                .searchmoves
                .iter()
                .copied()
                .filter(|&mv| board.is_legal(mv))
                .collect()
        } else {
            let mut moves = Vec::new();
            board.generate_moves(|piece_moves| {
                moves.extend(piece_moves);
                false
            });
            moves
        };

        if root_moves.is_empty() {
            let score = if board.checkers().is_empty() {
                0
            } else {
                evaluator.evaluate(board)
            };
            return SearchResult {
                best_move: None,
                score,
                depth: config.depth,
                nodes,
                stopped: false,
            };
        }

        let maximizing = board.side_to_move() == Color::White;
        let mut best_move: Option<Move> = None;
        let mut best_score: Option<Score> = None;
        let mut stopped = false;

        for mv in root_moves {
            if stop.is_stopped() {
                stopped = true;
                break;
            }

            let mut child = board.clone();
            child.play_unchecked(mv);

            let Some(score) = Self::minimax(&child, config.depth - 1, evaluator, stop, &mut nodes)
            else {
                stopped = true;
                break;
            };

            let is_better = match best_score {
                None => true,
                Some(cur) if maximizing => score > cur,
                Some(cur) => score < cur,
            };

            if is_better {
                best_score = Some(score);
                best_move = Some(mv);
            }
        }

        SearchResult {
            best_move,
            score: best_score.unwrap_or(0),
            depth: config.depth,
            nodes,
            stopped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::{INF, NEG_INF, SimpleEvaluator};
    use crate::search::SearchEngine;

    #[test]
    fn finds_mate_in_one_for_white() {
        // White rook on d1 can mate Black king on g8 with d1d8#.
        let board: Board = "6k1/5ppp/8/8/8/8/5PPP/3R2K1 w - - 0 1".parse().unwrap();
        let mut engine = SearchEngine::new(MinimaxSearcher::new(), SimpleEvaluator);

        let result = engine.search(&board, 1);
        assert_eq!(result.best_move, Some("d1d8".parse().unwrap()));
        assert_eq!(result.score, INF);
        assert!(!result.stopped);
    }

    #[test]
    fn finds_mate_in_one_for_black() {
        // Black rook on d8 can mate White king on g1 with d8d1#.
        let board: Board = "3r2k1/5ppp/8/8/8/8/5PPP/6K1 b - - 0 1".parse().unwrap();
        let mut engine = SearchEngine::new(MinimaxSearcher::new(), SimpleEvaluator);

        let result = engine.search(&board, 1);
        assert_eq!(result.best_move, Some("d8d1".parse().unwrap()));
        assert_eq!(result.score, NEG_INF);
        assert!(!result.stopped);
    }

    #[test]
    fn captures_hanging_queen_at_depth_two() {
        // White rook on e1 can capture an undefended Black queen on e5.
        let board: Board = "6k1/5ppp/8/4q3/8/8/5PPP/4R1K1 w - - 0 1".parse().unwrap();
        let mut engine = SearchEngine::new(MinimaxSearcher::new(), SimpleEvaluator);

        let result = engine.search(&board, 2);
        assert_eq!(result.best_move, Some("e1e5".parse().unwrap()));
        assert_eq!(result.score, 500);
        assert!(!result.stopped);
    }

    #[test]
    fn avoids_blundering_material_for_black() {
        // Black rook on a8 can capture an undefended White queen on a2; White has no recapture.
        let board: Board = "r5k1/5ppp/8/8/8/8/Q4PPP/6K1 b - - 0 1".parse().unwrap();
        let mut engine = SearchEngine::new(MinimaxSearcher::new(), SimpleEvaluator);

        let result = engine.search(&board, 2);
        assert_eq!(result.best_move, Some("a8a2".parse().unwrap()));
        assert_eq!(result.score, -500);
    }

    #[test]
    fn avoids_stalemate_when_winning() {
        // White king on f7, queen on e6 vs Black king on h8.
        // e6g6 stalemates Black (score 0), while e6e5 keeps White's +900 queen advantage.
        let board: Board = "7k/5K2/4Q3/8/8/8/8/8 w - - 0 1".parse().unwrap();
        let mut engine = SearchEngine::new(MinimaxSearcher::new(), SimpleEvaluator);
        let config = SearchConfig {
            depth: 2,
            searchmoves: vec!["e6g6".parse().unwrap(), "e6e5".parse().unwrap()],
        };

        let result = engine.search_with_config(&board, &config);
        assert_eq!(result.best_move, Some("e6e5".parse().unwrap()));
        assert_eq!(result.score, 900);
    }

    #[test]
    fn handles_terminal_positions_and_zero_depth() {
        let mut engine = SearchEngine::new(MinimaxSearcher::new(), SimpleEvaluator);

        // Depth 0 evaluates the position without searching moves.
        let start = Board::default();
        let res0 = engine.search(&start, 0);
        assert_eq!(res0.best_move, None);
        assert_eq!(res0.score, 0);
        assert_eq!(res0.depth, 0);
        assert_eq!(res0.nodes, 1);

        // Stalemate: Black king on h8, White king on f7, White queen on g6 (Black to move).
        let stalemate: Board = "7k/5K2/6Q1/8/8/8/8/8 b - - 0 1".parse().unwrap();
        let res_stale = engine.search(&stalemate, 2);
        assert_eq!(res_stale.best_move, None);
        assert_eq!(res_stale.score, 0);

        // Checkmate: White mated (Fool's mate).
        let mut mated = Board::default();
        for mv in ["f2f3", "e7e5", "g2g4", "d8h4"] {
            mated.play(mv.parse().unwrap());
        }
        let res_mate = engine.search(&mated, 2);
        assert_eq!(res_mate.best_move, None);
        assert_eq!(res_mate.score, NEG_INF);
    }

    #[test]
    fn respects_searchmoves_filter() {
        let board = Board::default();
        let mut engine = SearchEngine::new(MinimaxSearcher::new(), SimpleEvaluator);
        let target: Move = "e2e4".parse().unwrap();
        let config = SearchConfig {
            depth: 1,
            searchmoves: vec![target],
        };

        let result = engine.search_with_config(&board, &config);
        assert_eq!(result.best_move, Some(target));
    }

    #[test]
    fn stops_immediately_when_stop_signal_is_pre_set() {
        let mut searcher = MinimaxSearcher::new();
        let stop = StopSignal::new();
        stop.stop();

        let result = searcher.search(
            &Board::default(),
            &SimpleEvaluator,
            &SearchConfig::with_depth(4),
            &stop,
        );
        assert!(result.stopped);
        assert_eq!(result.nodes, 0);
        assert_eq!(result.best_move, None);
    }

    #[test]
    fn stops_in_progress_search_from_another_thread() {
        let mut searcher = MinimaxSearcher::new();
        let stop = StopSignal::new();
        let stop_clone = stop.clone();

        let stopper = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(15));
            stop_clone.stop();
        });

        // Depth 7 minimax on startpos would take minutes if not stopped.
        let result = searcher.search(
            &Board::default(),
            &SimpleEvaluator,
            &SearchConfig::with_depth(7),
            &stop,
        );
        stopper.join().unwrap();

        assert!(result.stopped);
        assert!(result.nodes > 0);
    }
}
