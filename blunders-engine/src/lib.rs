pub mod bitboard;
pub mod coretypes;
pub mod fen;
pub mod mailbox;
pub mod movegen;
pub mod position;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
