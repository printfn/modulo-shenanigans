use std::{
    ffi::CString,
    fs,
    io::{self, Write},
    mem, ptr,
};

use nix::{
    errno::{self, errno},
    libc::{close, mmap, open, MAP_PRIVATE, O_RDONLY, PROT_EXEC, PROT_READ, MAP_FAILED},
};

fn main() -> eyre::Result<()> {
    let path = "/tmp/modulo-shenanigans-bin";
    if fs::metadata(path).is_err() {
        println!("Could not open file '{path}', creating...");
        let start_time = std::time::Instant::now();
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
        io::stdout().flush()?;
        file.write_all(b"\xc3")?; // fallback ret
        file.flush()?;
        let end_time = std::time::Instant::now();
        println!("File created ({} seconds).", (end_time - start_time).as_secs());
    }

    let is_even = unsafe {
        let filesize = fs::metadata(path)?.len();
        println!("{path} has a size of {filesize} bytes");
        let c_path = CString::new(path.as_bytes())?;
        let fd = open(c_path.as_ptr(), O_RDONLY);
        if fd == -1 {
            return Err(errno::from_i32(errno()).into());
        }
        println!("Opened {path} as fd {fd}");
        let mapped_ptr = mmap(
            ptr::null_mut(),
            filesize as usize,
            PROT_EXEC | PROT_READ,
            MAP_PRIVATE,
            fd,
            0,
        );
        if mapped_ptr == MAP_FAILED {
            return Err(errno::from_i32(errno()).into());
        }
        println!("Mapped {path} at {mapped_ptr:?}");
        if close(fd) == -1 {
            return Err(errno::from_i32(errno()).into());
        }
        let function_ptr: fn(i32) -> i32 = mem::transmute(mapped_ptr);
        function_ptr
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
        let input: i32 = trimmed.parse()?;
        if is_even(input) != 0 {
            println!("{} is even", input);
        } else {
            println!("{} is odd", input);
        }
    }

    Ok(())
}
