pub mod arrayvec;
pub mod bitboard;
pub mod boardrepr;
pub mod coretypes;
pub mod evaluation;
pub mod fen;
pub(crate) mod movegen;
pub mod movelist;
pub mod moveorder;
pub mod perft;
pub mod position;
pub mod search;
pub mod transposition;
pub mod uci;
pub mod zobrist;

pub use fen::Fen;
pub use position::Position;
pub use transposition::TranspositionTable;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
