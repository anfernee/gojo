mod minimax;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cozy_chess::{Board, Move};

use crate::eval::{Evaluator, Score};

pub use minimax::MinimaxSearcher;

/// Thread-safe cooperative cancellation signal for stopping an in-progress search.
#[derive(Debug, Clone, Default)]
pub struct StopSignal {
    stopped: Arc<AtomicBool>,
}

impl StopSignal {
    pub fn new() -> Self {
        Self::default()
    }

    /// Request that any search observing this signal stops as soon as possible.
    pub fn stop(&self) {
        self.stopped.store(true, Ordering::Release);
    }

    /// Clear the stop flag before starting a fresh search.
    pub fn reset(&self) {
        self.stopped.store(false, Ordering::Release);
    }

    /// Returns `true` if a stop has been requested.
    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::Acquire)
    }
}

/// Parameters controlling a search invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchConfig {
    /// Maximum search depth in plies.
    pub depth: u32,
    /// Optional restriction of root moves to search (empty searches all legal moves).
    pub searchmoves: Vec<Move>,
}

impl SearchConfig {
    pub fn with_depth(depth: u32) -> Self {
        Self {
            depth,
            searchmoves: Vec::new(),
        }
    }
}

/// Outcome of a search invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    /// Best legal move found, if any.
    pub best_move: Option<Move>,
    /// Evaluation score in centipawns (positive favors White, negative favors Black).
    pub score: Score,
    /// Depth searched in plies.
    pub depth: u32,
    /// Total nodes visited during the search.
    pub nodes: u64,
    /// Whether the search terminated early due to a [`StopSignal`].
    pub stopped: bool,
}

/// Abstraction for a chess search algorithm (e.g., minimax, alpha-beta, MCTS).
pub trait Searcher: Send {
    fn search(
        &mut self,
        board: &Board,
        evaluator: &dyn Evaluator,
        config: &SearchConfig,
        stop: &StopSignal,
    ) -> SearchResult;
}

/// High-level search engine pairing a [`Searcher`] algorithm, an [`Evaluator`],
/// and a [`StopSignal`] to run and stop searches.
pub struct SearchEngine<S: Searcher, E: Evaluator> {
    searcher: S,
    evaluator: E,
    stop: StopSignal,
}

impl<S: Searcher, E: Evaluator> SearchEngine<S, E> {
    pub fn new(searcher: S, evaluator: E) -> Self {
        Self {
            searcher,
            evaluator,
            stop: StopSignal::new(),
        }
    }

    /// Returns a cloneable handle that can be used (including from another thread)
    /// to stop an active search.
    #[allow(dead_code)]
    pub fn stop_signal(&self) -> StopSignal {
        self.stop.clone()
    }

    /// Request that the current search stop.
    pub fn stop(&self) {
        self.stop.stop();
    }

    /// Search `board` to `depth` plies.
    #[allow(dead_code)]
    pub fn search(&mut self, board: &Board, depth: u32) -> SearchResult {
        self.search_with_config(board, &SearchConfig::with_depth(depth))
    }

    /// Search `board` using the provided [`SearchConfig`].
    pub fn search_with_config(&mut self, board: &Board, config: &SearchConfig) -> SearchResult {
        self.stop.reset();
        self.searcher
            .search(board, &self.evaluator, config, &self.stop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::SimpleEvaluator;

    struct StubSearcher {
        last_depth: u32,
    }

    impl Searcher for StubSearcher {
        fn search(
            &mut self,
            board: &Board,
            evaluator: &dyn Evaluator,
            config: &SearchConfig,
            stop: &StopSignal,
        ) -> SearchResult {
            self.last_depth = config.depth;
            if stop.is_stopped() {
                return SearchResult {
                    best_move: None,
                    score: 0,
                    depth: config.depth,
                    nodes: 0,
                    stopped: true,
                };
            }
            let mut first_move = None;
            board.generate_moves(|moves| {
                first_move = moves.into_iter().next();
                first_move.is_some()
            });
            SearchResult {
                best_move: first_move,
                score: evaluator.evaluate(board),
                depth: config.depth,
                nodes: 1,
                stopped: false,
            }
        }
    }

    #[test]
    fn stop_signal_shares_state_across_clones_and_threads() {
        let signal = StopSignal::new();
        let clone = signal.clone();
        assert!(!signal.is_stopped());

        let handle = std::thread::spawn(move || {
            clone.stop();
        });
        handle.join().unwrap();

        assert!(signal.is_stopped());
        signal.reset();
        assert!(!signal.is_stopped());
    }

    #[test]
    fn search_engine_delegates_to_searcher_and_evaluator() {
        let mut engine = SearchEngine::new(StubSearcher { last_depth: 0 }, SimpleEvaluator);
        let result = engine.search(&Board::default(), 4);

        assert_eq!(result.depth, 4);
        assert_eq!(result.score, 0);
        assert_eq!(result.nodes, 1);
        assert!(result.best_move.is_some());
        assert!(!result.stopped);
        assert_eq!(engine.searcher.last_depth, 4);
    }
}
