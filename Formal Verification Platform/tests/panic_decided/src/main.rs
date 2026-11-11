// A6: Panics whose receiver variant is statically known are decided directly:
// the safe variants leave no finding at all, and the failing variants are counterexamples.

pub fn known_none() -> u8 {
    let x: Option<u8> = None;
    x.unwrap()
}

pub fn known_err() -> u8 {
    let x: Result<u8, u8> = Err(1);
    x.unwrap()
}

pub fn known_ok_unwrap_err() -> u8 {
    let x: Result<u8, u8> = Ok(1);
    x.unwrap_err()
}

pub fn known_some() -> u8 {
    let x: Option<u8> = Some(5);
    x.unwrap()
}

pub fn known_ok() -> u8 {
    let x: Result<u8, u8> = Ok(5);
    x.unwrap()
}

pub fn known_err_unwrap_err() -> u8 {
    let x: Result<u8, u8> = Err(5);
    x.unwrap_err()
}

pub fn dead_site(flag: bool) -> u8 {
    if flag && !flag {
        let x: Option<u8> = None;
        x.unwrap()
    } else {
        0
    }
}

fn main() {
    let _ = known_some();
    let _ = known_ok();
    let _ = known_err_unwrap_err();
    let _ = dead_site(false);
}
