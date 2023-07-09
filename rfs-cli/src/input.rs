use std::io::Write;

pub fn read_stdin_buf<P>(prompt: P, buffer: &mut String) -> std::io::Result<usize>
where
    P: AsRef<str>
{
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let prompt_ref = prompt.as_ref();

    stdout.write_all(prompt_ref.as_bytes())?;
    stdout.flush()?;

    stdin.read_line(buffer)
}

pub fn read_stdin<P>(prompt: P) -> std::io::Result<String>
where
    P: AsRef<str>
{
    let mut buffer = String::new();

    read_stdin_buf(prompt, &mut buffer)?;

    Ok(buffer)
}

pub fn read_stdin_trimmed<P>(prompt: P) -> std::io::Result<String>
where
    P: AsRef<str>
{
    let given = read_stdin(prompt)?;

    Ok(given.trim().to_owned())
}
