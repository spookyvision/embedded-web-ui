use embedded_web_ui::BarData;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut vals = heapless::Vec::new();
    for i in 0u8..64 {
        vals.push(i).unwrap();
    }
    let data = BarData { id: 1, vals };
    let ser = postcard::to_allocvec_cobs(&data)?;
    println!("{}", ser.len());
    Ok(())
}
