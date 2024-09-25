pub enum Flow {
    Split,
    Duplicate,
    Success,
}

pub struct Node<const N: usize> {
    children : Vec<Node<N>>,
    keys : Vec<i64>,
}

impl<const N: usize> Node<N> {
    fn new() -> Node<N> {
        Node {children : vec![], keys : vec![]}
    }
    fn insert(&mut self, key: i64) -> Flow {
        //"index" will be the location in the current node that the given key can be inserted (i.e. it is greater than the previous and less than what is currently in that index, if anything is there)
        let Err( mut index) = self.keys.binary_search(&key) else {
            return Flow::Duplicate;
        };
        
        if self.children.len() > 0 {
            //inner or root node
            match self.children[index].insert(key) {
                Flow::Duplicate => return Flow::Duplicate,
                Flow::Success => return Flow::Success,
                Flow::Split =>
                    if self.children.len() == N {
                        //this node is full, check if one of the children 0..index-1 can manage a rotation
                        let mut i = index-1;
                        while i > 0 {
                            match self.children[i].insert(self.keys[i]) {
                                Flow::Success => 
                                    {
                                        self.keys.insert(index, key);
                                        self.keys.remove(i);
                                        return Flow::Success;
                                    },
                                _ => (),
                            }
                            i -= 1;
                        }
                        //if we've gotten here, this node can't manage the current key, we need to pass the split up.
                        return Flow::Split;
                    } else {
                        //this node is not full, we can split it up

                        //TODO! I'm not sure how to make this work yet, so this is just placeholder
                        let clipped = self.children[index].children.remove(0);
                        self.children.insert(index-1, clipped);
                        let clipped_key = self.children[index].keys.remove(0);
                        self.keys.insert(index, clipped_key);
                    }

            }
        } else {
            //leaf node
            if self.keys.len() == N-1 {
                //reached max keys, need to begin a split
                
                return Flow::Split;
            } else {
                //leaf can hold more keys, lets insert.

                // let (left,right) = self.keys.split_at_mut(index);
                // left[index] = key;
                // self.keys = left.iter().chain(right.iter()).map(|v| *v).collect::<Vec<i64>>().try_into().unwrap();

                self.keys.insert(index, key);
            }
        }

        return Flow::Success;
    }
    fn split(&mut self) {
        //this function is going to be a bitch to implement, saving for when I have more motivation
    }
}

pub struct BTree<const N: usize> {
    root : Node<N>,
    k : i32,
}

impl<const N: usize> BTree<N> {
    
}

// pub trait BTree<const N: usize>{

// }




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        // let result = add(2, 2);
        // assert_eq!(result, 4);
    }
}
