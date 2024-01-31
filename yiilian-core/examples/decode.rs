use yiilian_core::data::{decode, BencodeFrame as Frane};



fn main()
{
    let code = b"d1:t2:\xd2#1:y1:q1:q4:ping1:ad2:id20:\xa81Q\x14bc\x92_t@\x7f\x81\xd2\x1c\xf5-v0\xa9\x98ee";
    let frame: Frane = decode(code).unwrap();
    println!("{:?}", frame);
    
    
}