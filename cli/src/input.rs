use std::io::Write;

pub fn read_stdin<P>(prefix: P) -> std::io::Result<String>
where
    P: AsRef<str>
{
    let mut buffer = String::new();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let prefix_ref = prefix.as_ref();

    stdout.write_all(prefix_ref.as_bytes())?;
    stdout.flush()?;

    stdin.read_line(&mut buffer)?;

    Ok(buffer)
}
