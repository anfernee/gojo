use cozy_chess::Board;
use cozy_chess::{BitBoard, File, Rank, Square};

fn print_bitboard(bb: BitBoard) {
    for &rank in Rank::ALL.iter().rev() {
        print!("{} ", rank as u8 + 1);
        for &file in &File::ALL {
            let sq = Square::new(file, rank);
            let ch = if bb.has(sq) { '1' } else { '.' };
            print!("{} ", ch);
        }
        println!();
    }
    println!("  a b c d e f g h");
}

fn main() {

    let mut board: Board = "1r4r1/pbpknp1p/1b3P2/8/8/B1PB1q2/P4PPP/3R2K1 w - - 0 22".parse().unwrap();
    assert_eq!(board.checkers(), BitBoard::EMPTY);
    
    board.play("d3f5".parse().unwrap());
    print_bitboard(board.checkers());
}
