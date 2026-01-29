use clorus_syntax::lexer::Lexer;

fn main() {
    let input = "#(* % 2)";
    println!("Tokenizing: {}", input);

    let mut lexer = Lexer::new(input);
    match lexer.tokenize() {
        Ok(tokens) => {
            println!("Tokens:");
            for token in &tokens {
                println!("  {:?}", token);
            }
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
