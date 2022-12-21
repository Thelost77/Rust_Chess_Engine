use args::{Args, ArgsError};
use chess::{get_rank, Board, BoardStatus, ChessMove, Color, MoveGen, Piece, ALL_RANKS};
use getopts::Occur;
use std::env;
use std::io::BufRead;
use std::str::FromStr;
use std::time::Instant;

mod benchmarks;
mod piece_values;

const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const DEFAULT_DEPTH: i64 = 4;

const PROGRAM_DESC: &str = "A Chess Engine built in Rust";
const PROGRAM_NAME: &str = "Scacchi";

fn calc_piece_value(pc_idx: usize, sq_idx: usize, colour: Option<Color>) -> i64 {
    match colour {
        Some(Color::White) => {
            let sq_value = piece_values::PIECE_SQUARES[pc_idx][sq_idx];
            return -(piece_values::PIECE_VALS[pc_idx] + sq_value);
        }
        Some(Color::Black) => {
            let sq_value = piece_values::PIECE_SQUARES[pc_idx][63 - sq_idx];
            return -(piece_values::PIECE_VALS[pc_idx] + sq_value);
        }
        None => 0    
    }
}

fn calc_pieces_value(board: &Board) -> i64 {
    let mut result = 0;
    for pc_idx in 0..6 {
        let pc_type = piece_values::PIECES[pc_idx];
        let bboard = *board.pieces(pc_type);
        for square in bboard {
            let sq_idx = square.to_index();
            result += calc_piece_value(pc_idx, sq_idx, board.color_on(square));
        }
    }
    result
}

fn calc_board_value(board: &Board) -> i64 {
    let w_move = board.side_to_move() == Color::White;
    let result = match board.status() {
        BoardStatus::Ongoing => calc_pieces_value(board),
        BoardStatus::Stalemate => 0,
        BoardStatus::Checkmate => {
            if w_move {
                20000
            } else {
                -20000
            }
        }
    };

    result
}

fn alpha_beta(
    board: &Board,
    depth: i8,
    is_max: bool,
    alpha: i64,
    beta: i64,
    total: &mut i64,
) -> i64 {
    if (depth == 0) || (board.status() != BoardStatus::Ongoing) {
        *total += 1;
        let val = calc_board_value(board);
        return val;
    }

    let mut alpha = alpha;
    let mut beta = beta;

    if is_max {
        let mut best_value = i64::MIN;
        let moves = MoveGen::new_legal(&board);
        let mut result_board = chess::Board::default();
        for mv in moves {
            board.make_move(mv, &mut result_board);

            let value = alpha_beta(&result_board, depth - 1, false, alpha, beta, total);
            best_value = std::cmp::max(value, best_value);

            alpha = std::cmp::max(alpha, best_value);
            if beta <= alpha {
                break;
            }
        }
        return best_value;
    } else {
        let mut best_value = i64::MAX;
        let moves = MoveGen::new_legal(&board);
        let mut result_board = chess::Board::default();
        for mv in moves {
            board.make_move(mv, &mut result_board);

            let value = alpha_beta(&result_board, depth - 1, true, alpha, beta, total);
            best_value = std::cmp::min(value, best_value);

            beta = std::cmp::min(beta, best_value);
            if beta <= alpha {
                break;
            }
        }
        return best_value;
    }
}

fn show_board(board: Board) {
    for (&rank, lbl) in ALL_RANKS.iter().zip("12345678".chars()) {
        print!("{}", lbl);
        print!(" ");
        for sq in get_rank(rank) {
            let piece = board.piece_on(sq);
            let sq_char = match board.color_on(sq) {
                Some(Color::Black) => match piece {
                    Some(Piece::King) => "♚",
                    Some(Piece::Queen) => "♛",
                    Some(Piece::Rook) => "♜",
                    Some(Piece::Bishop) => "♝",
                    Some(Piece::Knight) => "♞",
                    Some(Piece::Pawn) => "♟",
                    _ => "?",
                },
                Some(Color::White) => match piece {
                    Some(Piece::King) => "♔",
                    Some(Piece::Queen) => "♕",
                    Some(Piece::Rook) => "♖",
                    Some(Piece::Bishop) => "♗",
                    Some(Piece::Knight) => "♘",
                    Some(Piece::Pawn) => "♙",
                    _ => "?",
                },
                _ => ".",
            };
            print!("{} ", sq_char);
        }
        print!("\n");
    }
    println!("  a b c d e f g h");
}

fn find_best_move(board: &Board, depth: i8) -> Option<ChessMove> {
    let black_move = board.side_to_move() == Color::Black;
    let moves = MoveGen::new_legal(board);

    let mut best_value;
    let mut best_move = None;

    let is_better = {
        if black_move {
            best_value = i64::MIN;
            |x: i64, y: i64| -> bool { x > y }
        } else {
            best_value = i64::MAX;
            |x: i64, y: i64| -> bool { x < y }
        }
    };

    let mut total = 0;
    for mv in moves {
        let mut new_board = Board::default();
        board.make_move(mv, &mut new_board);
        let value = alpha_beta(
            &new_board,
            depth,
            black_move,
            i64::MIN,
            i64::MAX,
            &mut total,
        );
        if is_better(value, best_value) {
            best_value = value;
            best_move = Some(mv);
        }
    }

    best_move
}

fn parse(input: &Vec<String>) -> Result<(bool, bool, bool, String, i8), ArgsError> {
    let mut args = Args::new(PROGRAM_NAME, PROGRAM_DESC);
    args.flag("h", "help", "Print the usage menu");
    args.flag("i", "interactive", "Run in interactive mode");
    args.flag("s", "selfplay", "Run in self play mode");
    args.flag("b", "bench", "Run benchmark");
    args.option(
        "d",
        "depth",
        "Set the depth of tree search - default 4",
        "DEPTH",
        Occur::Req,
        Some(DEFAULT_DEPTH.to_string()),
    );
    args.option(
        "f",
        "fen",
        "The state of the game as a FEN",
        "FEN",
        Occur::Optional,
        Some(STARTING_FEN.to_string()),
    );
    args.parse(input)?;

    let is_help = args.value_of("help")?;
    if is_help {
        args.full_usage();
    };
    let is_interactive = args.value_of("interactive")?;
    let is_selfplay = args.value_of("selfplay")?;
    let run_benchmark = args.value_of("bench")?;
    let fen_str = args.value_of("fen")?;
    let play_count = args.value_of::<String>("depth")?.parse::<i8>().unwrap();
    println!("Depth: {}", play_count);
    Ok((
        is_interactive,
        is_selfplay,
        run_benchmark,
        fen_str,
        play_count,
    ))
}

fn exec_ai_turn(board: &mut Board, ply_count: i8) {
    match find_best_move(board, ply_count) {
        Some(n) => *board = board.make_move_new(n),
        None => {
            println!("Error!! No move found")
        }
    }
    println!("--------------------");
    show_board(*board);
}

fn exec_user_turn(board: &mut Board) {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let s = match line {
            Ok(l) => l,
            Err(_) => "".to_string(),
        };

        if let Ok(mv) = ChessMove::from_san(&board, &s) {
            *board = board.make_move_new(mv);
            break;
        } else {
            println!("Invalid Move");
        }

        println!("--------------------");
        show_board(*board);
    }
}

fn interactive_loop(mut board: Board, ply_count: i8) {
    let mut ai_turn = true;
    loop {
        match board.status() {
            BoardStatus::Ongoing => {
                if ai_turn {
                    exec_ai_turn(&mut board, ply_count);
                } else {
                    println!("Your turn...");
                    exec_user_turn(&mut board);
                }
                ai_turn = !ai_turn;
            }
            BoardStatus::Stalemate => {
                println!("Stalemate...");
                return;
            }
            BoardStatus::Checkmate => {
                println!("Checkmate!!");
                return;
            }
        }
    }
}

fn self_play_loop(mut board: Board, ply_count: i8) {
    loop {
        if board.status() == BoardStatus::Ongoing {
            exec_ai_turn(&mut board, ply_count)
        } else {
            return;
        }
    }
}

fn run_benchmark() {
    println!("name\tdepth\tduration");
    for (name, fen) in benchmarks::CASES {
        let start = Instant::now();
        match Board::from_str(fen) {
            Ok(board) => {
                for &depth in benchmarks::DEPTHS {
                    find_best_move(&board, depth);
                    let duration = start.elapsed().as_millis();
                    println!("{}\t{}\t{}", name, depth, duration);
                }
            }
            Err(_) => {}
        }
    }
}

fn main() {
    println!("Scacchi !!");

    let args: Vec<String> = env::args().collect();
    let (is_interactive, is_selfplay, run_bench, fen_str, play_count) = parse(&args).unwrap();

    if run_bench {
        run_benchmark();
        return;
    }

    let board = match Board::from_str(fen_str.as_str()) {
        Ok(b) => b,
        Err(_) => {
            println!("Bad FEN");
            return;
        }
    };

    if is_selfplay {
        self_play_loop(board, play_count);
        println!("Good Game!");
        return;
    }

    if !is_interactive {
        match find_best_move(&board, play_count) {
            Some(n) => {
                println!("Best Move: {}", n)
            }
            None => {
                println!("Error!! No move found!")
            }
        }
    } else {
        interactive_loop(board, play_count);
    }
}
