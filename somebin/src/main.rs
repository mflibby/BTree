
fn main()  {
    let mut test = vec![1,2,3,5,6];
    let res = test.binary_search(&4);
    let index = match res { Ok(i) => i, Err(i) => i};
    // let (l,r) = test.split_at_mut(index);
    // l.push(4);
    test.insert(5, 4);
    println!("{:?}", test);
}