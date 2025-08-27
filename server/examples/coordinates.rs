use proc_mem::{ProcMemError, Process};

fn main() -> Result<(), ProcMemError> {
    let minecraft = Process::with_name("Minecraft.Windows.exe")?;
    println!("Minecraft PID: {}", minecraft.pid());
    let module = minecraft.module("Minecraft.Windows.exe")?;
    let y_coordinate: Result<f32, ProcMemError> =
        minecraft.read_mem::<f32>(module.base_address() + 0x00000EF0);
    println!("Y Coordinate: {:?}", y_coordinate);

    Ok(())
}
// 1D6E3174E30

//1D6E30D0bcp47mrm.dll+2E304
