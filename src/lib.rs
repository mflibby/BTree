use std::{array, default, fmt::{Debug, Display}, ptr::null};

//Notes/lingo
//1. BTree<T,M,K> is a btree of order M whose nodes store K = (M-1) keys of type T and M children
//2. ideally, implementation should work for any M in [2,~2**64] (though the upper bound would probably always be very slow)
//3. the mutation of a nodes keys and children needs to be ---FAST--- (without forcing our way out of rusts rules)
//4. Vec does not impl Copy
//5. Vec is stored on the heap, primitive arrays (i.e. [T; n]) on the stack.
//5a. For larger BTrees, we probably don't want/can't have the whole thing on the stack
//5b. However, we change the contents of the keys/children collections a lot
//    and those mutations are generally "remove [n...at) from this collection, pass it to the parent node"
//    my ignorance of ownership rules aside, we cannot change the size of an array, so we would need options or null
//    even if we can do that, I do not think we could generate a btree instance with its definite size from a file (i.e., the btree order would need to be known at compile time.)
//    ---I think we NEED to use Vec (or, atleast, not arrays).
//    ~addendum~ we may be able to bake in btree sizes in order to use arrays (not to say that is necessarily desirable, just possible).
//6. Regardless of Vec vs Array, Nodes MUST be on the stack

//A. I've implemented a tree as an adjacency list before, but never as a nested-pointer where the tree instance itself only contains a pointer to the node


#[derive(Debug)]
pub enum Flow<T, const M: usize, const K: usize> {
    //splitting should probably pass the pointers necessary to create a new node, instead of the new node itself (I keep going back and forth on this).
    Split (Vec<T>, Vec<Node<T,M,K>>, T, Vec<usize>), //(keys to make a new node, children-to-be of that new node, split point key, index in vec of originally inserted key)
    Duplicate,
    Success (Vec<usize>), //(reverse of insertion traversal)
    LeafSplit (Vec<T>, T, usize), //(same as split, sans children-to-be)
    NoImpl,
}

#[derive(Clone, Debug)]
/// M is the knuth order, K is M-1 (max number of keys)
pub struct Node<T, const M: usize, const K: usize> {
    children : Vec<Node<T, M, K>>,
    keys : Vec<T>,
    is_root : bool,
}

impl<T, const M: usize, const K: usize> Display for Node<T, M, K> where T: std::fmt::Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{:?}", self.keys);
        for i in &self.children {
            write!(f, "{}", i);
        }
        Ok(())
    }
}

// impl<T,M,K> Copy for Node<T, M, K> where T: Copy {}

// impl<T> Copy for Vec<T> where T: Copy {}
// impl<T, const M: usize, const K: usize> Copy for Box<Node<T,K,M>> {}
// impl<const N: usize> Clone for Node<N> {
//     fn clone(&self) -> Self {
//         *self
//     }
// }

/// All array mutations will occur through here, hopefully making it easier to improve in the future
fn mutate_array<T, const M: usize>(mut arr: [T; M], index: usize, elem: T) {
    arr[index] = elem;
}

fn swap_keys<T>(mut arr: Vec<T>, removal_index: usize, insertion_index: usize, elem: T) -> T {
    arr.insert(insertion_index, elem);
    arr.remove(removal_index)
}

fn make_leaf<T, const M: usize, const K: usize>(keys: Vec<T>) -> Node<T,M,K> {
    Node {children : vec![], keys : keys, is_root : false}
}

fn make_inner_node<T, const M: usize, const K: usize>(keys: Vec<T>, children: Vec<Node<T,M,K>>) -> Node<T,M,K> {
    Node {children : children, keys : keys, is_root : false}
}

impl<T : Ord + Debug, const M: usize, const K: usize> Node<T,M,K> {
    fn new() -> Node<T,M,K> {
        Node {children : vec![], keys : vec![], is_root : false}
    }

    fn has_children(&self) -> bool {
        self.children.len() > 0
    }

    ///Measure if a particular node has a certain decendency index.
    ///Decendency is the count of progenious generations beneath a particular node
    /// 
    ///E.g. a node with children who have no children would have DI = 1, a node with no children has a DI = 0
    /// 
    ///The root node of a tree with height N has a DI = N - 1 
    /// 
    ///For BTree of height N, worst case is O(N)
    fn has_decendency(&self, dec: usize) -> bool {
        match dec {
            0 => !self.has_children(),
            _ => self.has_children() && self.children[0].has_decendency(dec-1)
        }
    }

    ///Calculate the Decendency Index of this node.
    /// 
    /// Probably don't use this... just here for reference.
    /// 
    /// Also, this isn't proper recursion, should probably lift this into a static method.
    fn DI(&self) -> i64 {
        if !self.has_children() {
            return 0;
        } else {
            return self.children[0].DI();
        }
    }

    fn insert(&mut self, key: T) -> Flow<T, M, K> {
        //"index" will be the location in the current node that the given key can be inserted (i.e. it is greater than the previous and less than what is currently in that index, if anything is there)
        let Err(index) = self.keys.binary_search(&key) else {
            return Flow::Duplicate;
        };
        
        //TODO!: this always works for K%2=0, placeholder to deal with odd K later (maybe)
        //the key splindex (splitting index) is the index of the chosen median index + 1
        //with even K, this will always be one to the right of the central element after inserting the key (i.e. the center element +1 of a K+1 long collection) 
        let key_splindex = K/2 + 1;

        //leaf node
        if !self.has_children() {
            //TODO!: it might be faster to have the keys be VecDeque and split such that the new key can be pushed onto the back of the collection being left behind (though that will certainly increase the number of splitting necessary insert over insert)
            self.keys.insert(index, key);
            match self.keys.len() {
                k if k == M => {
                    //after insert, node has K+1 keys (needs to split). [0,1,2,3] + 4 => [0,1,2,3,4] => keep [0,1] and return ([3,4], 2)
                    //split off the keys for the new node, and then pop the end off what remains in self.keys (presumably faster than splitting off K/2 since then we would have to remove the 0th key from that later anyway).
                    if !self.is_root {
                        return Flow::LeafSplit (self.keys.split_off(key_splindex), self.keys.pop().expect("keys.pop() should never fail here... M must be 0"), index);
                    } else {
                        let right_new = make_inner_node(self.keys.split_off(key_splindex), vec![]);
                        let left_new = make_inner_node(self.keys.drain(..key_splindex-1).collect(), vec![] );
                        
                        self.children.push(left_new);
                        self.children.push(right_new);

                        return Flow::Success(vec![(index+key_splindex) % key_splindex, (index/key_splindex).into()]);
                    }
                },
                _ => {
                    //after insert
                    return Flow::Success (vec![index])
                },
            }
        } else {
            //inner or root node
            match self.children[index].insert(key) {
                //We are guarenteed that children[index] exists since children.len() == keys.len() + 1,
                //and the bin search (source of "index") will at most return keys.len() + 1 
                Flow::Duplicate => return Flow::Duplicate,
                Flow::Success (mut i) => {
                    i.push(index);
                    return Flow::Success (i);
                },
                Flow::LeafSplit(new_node_keys, key, lindex) => {
                    //self.DI() == 1 (last inner node) && leaf was full.
                    //TODO! LeafSplit and Split share the same general structure, functionality should be abstracted (or perhaps squashed to one case)
                    
                    self.keys.insert(index, key);
                    if self.keys.len() == M {
                        //innernode/root is full, handle:
                        // let new_node = make_inner_node(self.keys.split_off(key_splindex), );
                        if !self.is_root {
                            return Flow::Split (self.keys.split_off(key_splindex), self.children.split_off(key_splindex), self.keys.pop().expect("keys.pop() should never fail here... M must be 0?"), vec![lindex, index]);
                        } else {

                            print!("DEBUG - LEAFSPLIT: {:?} =>", self.keys);
                            self.children.insert(index+1, make_leaf(new_node_keys));
                            let right_new = make_inner_node(self.keys.split_off(key_splindex), self.children.split_off(key_splindex));
                            let left_new = make_inner_node(self.keys.drain(..key_splindex-1).collect(), self.children.drain(..key_splindex).collect() );

                            println!("{:?} - {:?}", left_new.keys, right_new.keys);
                            println!("{:?} - {:?}", left_new.children, right_new.children);
                            self.children.push(left_new);
                            self.children.push(right_new);

                            return Flow::Success(vec![(index+key_splindex) % key_splindex, (index/key_splindex).into()]);
                        }
                    } else {
                        self.children.insert(index+1, make_leaf(new_node_keys));
                        return Flow::Success(vec![lindex, index]);
                    }
                },
                Flow::Split (new_node_keys, new_node_children, key, mut trace) => {
                    //self.DI() != 1 && non-leaf child was full
                    
                    trace.push(index);
                    //TODO!: This is a bit lazy, since when children.len() == M we are just going to split these two apart again. For now, it does what we need for both cases (and is required for when children.len() < M)
                    //add the removed median
                    self.keys.insert(index, key);

                    let new_node_in = make_inner_node(new_node_keys, new_node_children);
                    //add the split node
                    self.children.insert(index + 1, new_node_in);
                    
                    if self.keys.len() == M {
                        //this node was already full, do another split
                        // let new_node = make_inner_node(self.keys.split_off(key_splindex), self.children.split_off(key_splindex));
                        if !self.is_root {
                            return Flow::Split( self.keys.split_off(key_splindex)
                                              , self.children.split_off(key_splindex)
                                              , self.keys.pop().expect("keys.pop() should never fail here... M must be 0?")
                                              , trace
                                            )
                        } else {
                            //[0,1,2,3,4] => [0,1] [2] [3,4]
                            let right_new = make_inner_node(self.keys.split_off(key_splindex), self.children.split_off(key_splindex));
                            let left_new = make_inner_node(self.keys.drain(..key_splindex).collect(), self.children.drain(..key_splindex).collect() );
                            
                            self.children.push(left_new);
                            self.children.push(right_new);

                            return Flow::Success(trace);
                        }
                    } else {
                        return Flow::Success(trace);
                    }
                }
                _ => return Flow::NoImpl,
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct BTree<T : Ord, const M: usize, const K: usize> {
    root : Node<T,M,K>,
}

impl<T : Ord + Debug, const M: usize, const K: usize> BTree<T, M, K> {
    pub fn new() -> BTree<T, M, K>{
        BTree {
            root : Node::<T, M, K> {children : vec![], keys : vec![], is_root : true}//Box::new(Node::<T, M, K> {children : vec![], keys : vec![]})
        }
    }

    // pub fn replace_root(&mut self, node: Box<Node<T,M,K>>) {
    //     self.root = node;
    // }
    pub fn insert(&mut self, key : T) -> Flow<T,M,K> {
        self.root.insert(key)
    }
}

// pub fn insert<T : Ord, const M: usize, const K: usize>(mut tree : BTree<T, M, K>, key: T) -> Flow<T, M, K> {
//     match tree.root.insert(key) {
//         Flow::Split(new_node, root_key, mut trace) => {
//             let mut new_root = make_leaf(vec![root_key]);
//             let root = tree.root;
//             new_root.children.push(root);
//             new_root.children.push(new_node);

//             tree.root = Box::new(new_root);
//             trace.push(1);
//             Flow::Success(trace)
//         }
//         a => a
//     }
// }
// pub trait BTree<const N: usize>{

// }




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut btree = BTree::<i64, 3, 2>::new();
        match btree.insert(32) {
            Flow::Success(result) => assert_eq!(result, vec![0]),
            a => panic!("Expected Success, got {:?}", a)
        }
    }

    #[test]
    fn fails_on_duplicate() {
        let mut btree = BTree::<i64, 3, 2>::new();
        btree.insert(32);
        match btree.insert(32) {
            Flow::Duplicate => (),
            a => panic!("Expected Duplicate, got {:?}", a)
        }
    }
    
    #[test]
    fn insert_10_keys() {
        let mut btree = BTree::<i32, 3, 2>::new();
        for i in 1..10 {
            match btree.insert(i) {
                Flow::Success(result) => println!("TRACE:{:?}\n{}", result, btree.root),
                a => panic!("Expected Success, got {:?}", a)
            }
        }
    }
}
