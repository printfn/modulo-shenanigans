use std::{
    ffi, fs,
    io::{self, Write},
    mem,
    os::fd::AsRawFd,
    ptr, time,
};

use nix::{
    errno::Errno,
    libc::{mmap, MAP_FAILED, MAP_PRIVATE, PROT_EXEC, PROT_READ},
};

fn main() -> eyre::Result<()> {
    let path = "/tmp/modulo-shenanigans-bin";
    if fs::metadata(path).is_err() {
        println!("Could not open file '{path}', creating...");
        let start_time = time::Instant::now();
        let file = fs::File::create(path)?;
        let mut file = io::BufWriter::new(file);
        file.write_all(b"\x31\xc0")?; // xor eax, eax
        let mut last_percentage = -1;
        for i in 0..=u32::MAX {
            file.write_all(b"\x81\xf9")?; // cmp ecx, i
            file.write_all(&i.to_le_bytes()[..])?;
            if i % 2 == 0 {
                file.write_all(b"\x75\x03")?; // jne +3
                file.write_all(b"\xff\xc0")?; // inc eax
            } else {
                file.write_all(b"\x75\x01")?; // jne +1
            }
            file.write_all(b"\xc3")?; // ret
            let percentage = (i as f64 / u32::MAX as f64 * 100.0) as i32;
            if percentage != last_percentage {
                print!(".");
                io::stdout().flush()?;
                last_percentage = percentage;
            }
        }
        println!();
        file.write_all(b"\xc3")?; // fallback ret
        file.flush()?;
        let end_time = time::Instant::now();
        println!(
            "File created ({} seconds).",
            (end_time - start_time).as_secs()
        );
    }

    let filesize = fs::metadata(path)?.len();
    println!("{path} has a size of {filesize} bytes");
    let file = fs::File::open(path)?;
    let fd = file.as_raw_fd();
    println!("Opened {path} as fd {fd}");

    let is_even = unsafe {
        let mapped_ptr = mmap(
            ptr::null_mut(),
            filesize as usize,
            PROT_EXEC | PROT_READ,
            MAP_PRIVATE,
            fd,
            0,
        );
        if mapped_ptr == MAP_FAILED {
            return Err(Errno::last().into());
        }
        println!("Mapped {path} at {mapped_ptr:?}");
        mem::transmute::<*mut ffi::c_void, fn(u32) -> bool>(mapped_ptr)
    };

    loop {
        let mut input = String::new();
        print!("Enter a number: ");
        io::stdout().flush()?;
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        if trimmed.is_empty() || trimmed == "exit" || trimmed == "quit" {
            break;
        }
        let input: u32 = match trimmed.parse() {
            Ok(input) => input,
            Err(e) => {
                println!("Error: {}", e);
                continue;
            }
        };
        if is_even(input) {
            println!("{} is even", input);
        } else {
            println!("{} is odd", input);
        }
    }

    Ok(())
}
