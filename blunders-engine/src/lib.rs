pub mod bitboard;
pub mod boardrepr;
pub mod coretypes;
pub mod evaluation;
pub mod fen;
pub(crate) mod movegen;
pub mod perft;
pub mod position;

pub use position::Position;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
