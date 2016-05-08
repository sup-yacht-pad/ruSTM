mod transaction;
mod variable;
mod result;

#[cfg(test)]
mod test;

extern crate rand;
extern crate time;  

pub use variable::TVar;
pub use transaction::Transaction;
pub use result::*;
use std::sync::{Arc, Mutex};
use std::thread;
use rand::{thread_rng, Rng};
use time::*;

pub fn retry<T>() -> StmResult<T> {
    Err(StmError::Retry)
}

pub fn atomically<T, F>(f: F) -> T
where F: Fn(&mut Transaction) -> StmResult<T>
{
    Transaction::run(f)
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
struct BstNode {
    val: i32,
    l: Option<Box<BstNode>>,
    r: Option<Box<BstNode>>,
}

impl BstNode {
    fn new(new_val: i32) -> BstNode {
        BstNode {
            val: new_val,
            l: None,
            r: None,
        }
    }

    fn insert(&mut self, new_val: i32) {
        if self.val == new_val {
            return
        }
        let target_node = if new_val < self.val { &mut self.l } else { &mut self.r };
        match target_node {
            &mut Some(ref mut subnode) => subnode.insert(new_val),
            &mut None => {
                let new_node = BstNode { val: new_val, l: None, r: None };
                let boxed_node = Some(Box::new(new_node));
                *target_node = boxed_node;
            }
        }
    }

    fn size(&mut self) -> i32 {
        match (&mut self.l, &mut self.r) {
            (&mut None, &mut None) => 1,
            (&mut Some(ref mut subnodel), &mut Some(ref mut subnoder))=> subnodel.size() + subnoder.size() + 1,
            (&mut None, &mut Some(ref mut subnoder))=> subnoder.size() + 1,
            (&mut Some(ref mut subnodel), &mut None)=> subnodel.size() + 1,
        }
    }
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
struct Bst {
    root: Option<Box<BstNode>>,
}

impl Bst {
    pub fn new() -> Bst {
        Bst { root: None }
    }

    pub fn insert(&mut self, new_val: i32) {
        match self.root {
            None => self.root = Some(Box::new(BstNode::new(new_val))),
            Some(ref mut r) => r.insert(new_val),
        }
    }

    pub fn size(&mut self) -> i32 {
        match self.root {
            None => 0,
            Some(ref mut subnode) => subnode.size(),
        }
    }
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
struct LlNode {
    val: i32,
    next: Option<Box<LlNode>>,
}

impl LlNode {
    fn new(new_val: i32) -> LlNode {
        LlNode {
            val: new_val,
            next: None,
        }
    }

    fn insert(&mut self, new_val: i32) {
        if self.val == new_val {
            return
        }
        let target_node = &mut self.next;
        let (new, flag) = match target_node {
            &mut Some(ref mut r) => {let cmp = r.val;
                                    let retval = if cmp > new_val {let mut new = LlNode::new(new_val);
                                                       new.next = Some(Box::new(*(r.clone())));
                                                       let mut newer = Some(Box::new(new));                                                       
                                                       (newer, true)} 
                                    else {r.insert(new_val);
                                          (Some(Box::new(LlNode::new(0))), false)};
                                    retval},
            &mut None => {
                let new_node = LlNode { val: new_val, next: None };
                let boxed_node = Some(Box::new(new_node));
                (boxed_node, true)
            }
        };
        if flag {*target_node = new} else {()};
    }

    fn len(&mut self) -> i32 {
        match &mut self.next {
            &mut None => 1,
            &mut Some(ref mut subnode)=> subnode.len() + 1,
        }
    }
}

#[derive(PartialEq)]
#[derive(Clone)]
#[derive(Debug)]
struct Ll {
    root: Option<Box<LlNode>>,
}

impl Ll {
    pub fn new() -> Ll {
        Ll { root: None }
    }

    pub fn insert(&mut self, new_val: i32) {
        let (new, flag) = match self.root {
            None => (Some(Box::new(LlNode::new(new_val))), true),
            Some(ref mut r) => { let cmp = r.val;
                                 let retval = if cmp > new_val { let mut new = LlNode::new(new_val);
                                                    new.next = Some(Box::new(*(r.clone())));
                                                    let mut newer = Some(Box::new(new));
                                                    (newer, true)
                                                    } 
                                 else {r.insert(new_val);
                                        (Some(Box::new(LlNode::new(0))), false)};
                                        retval},
        };
        if flag {self.root = new} else {()};
    }

    pub fn len(&mut self) -> i32 {
        match self.root {
            None => 0,
            Some(ref mut subnode) => subnode.len(),
        }
    }
}

//small shall be defined as <50 nodes, medium as 50-100 and large as >100 nodes

fn bst_insertion_backbone_small_with_all_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(x + 1000);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn bst_insertion_backbone_small_with_all_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                cur.insert(y + 1000);
             }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 25);
}

fn bst_insertion_backbone_small_with_all_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            newb.insert(y + 1000);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 25);
}

fn bst_insertion_backbone_medium_with_all_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(x + 1000);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 85);
}

fn bst_insertion_backbone_medium_with_all_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                cur.insert(x + 1000);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 85);
}

fn bst_insertion_backbone_medium_with_all_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            newb.insert(x + 1000);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 85);
}


fn bst_insertion_backbone_large_with_all_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(x + 5000);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 125);
}


fn bst_insertion_backbone_large_with_all_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                cur.insert(x + 5000);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 125);
}

fn bst_insertion_backbone_large_with_all_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            newb.insert(x + 5000);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 125);
}

fn bst_insertion_backbone_small_with_half_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        let mut y = if x < 48 {x - 48} else {x + 1000};
        b.insert(y);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 25);
}


fn bst_insertion_backbone_small_with_half_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..15 {
        b.insert(x);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                let z = if y < 48 {y - 48} else {y + 15};
                cur.insert(z);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 25);
}


fn bst_insertion_backbone_small_with_half_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..15 {
        b.insert(x);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            let z = if y < 48 {y - 48} else {y + 15};
            newb.insert(z);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 25);
}

fn bst_insertion_backbone_medium_with_half_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        let mut y = if x < 48 {x - 48} else {x + 1000};
        b.insert(y);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 85);
}

fn bst_insertion_backbone_medium_with_half_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                let z = if y < 48 {y - 48} else {y + 1000};
                cur.insert(z);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 85);
}

fn bst_insertion_backbone_medium_with_half_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            let z = if y < 48 {y - 48} else {x + 1000};
            newb.insert(z);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 85);
}

fn bst_insertion_backbone_large_with_half_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        let mut y = if x < 48 {x - 48} else {x + 5000};
        b.insert(y);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 125);
}

fn bst_insertion_backbone_large_with_half_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                let z = if y < 48 {y - 48} else {y + 5000};
                cur.insert(z);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 125);
}

fn bst_insertion_backbone_large_with_half_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            let z = if y < 48 {y - 48} else {x + 5000};
            newb.insert(z);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 125);
}

fn bst_insertion_backbone_small_with_no_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x * 2);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(x * 2 + 1);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 25);
}

fn bst_insertion_backbone_small_with_no_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..15 {
        b.insert(x * 2);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(y * 2 + 1);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 25);
}

fn bst_insertion_backbone_small_with_no_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..15 {
        b.insert(x * 2);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            newb.insert(y * 2 + 1);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 25);
}

fn bst_insertion_backbone_medium_with_no_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x * 2);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(x * 2 + 1);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 85);
}

fn bst_insertion_backbone_medium_with_no_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x * 2);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(y * 2 + 1);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 85);
}

fn bst_insertion_backbone_medium_with_no_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..1000 {
        b.insert(x * 2);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            newb.insert(y * 2 + 1);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 85);
}

fn bst_insertion_backbone_large_with_no_collisions_seq() {
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x * 2);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(x * 2 + 1);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(b.size(), 125);
}

fn bst_insertion_backbone_large_with_no_collisions_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x * 2);
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(y * 2 + 1);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().size(), 125);
}

fn bst_insertion_backbone_large_with_no_collisions_single_lock() {
    let mut b = Bst::new();
    for x in 0..5000 {
        b.insert(x * 2);
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            newb.insert(y * 2 + 1);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.size(), 125);
}

fn bst_insertion_random_small_seq() {
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        b.insert(rng.gen_range(0, 1500));
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(rng.gen_range(0, 1500));
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn bst_insertion_random_small_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..15 {
        b.insert(rng.gen_range(0, 38));
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut rng = thread_rng();
             let mut cur = try!(newvar.read(trans));
             for _ in (8*x)..(8*(x+1)) {
                 cur.insert(rng.gen_range(0, 38));
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}


fn bst_insertion_random_small_single_lock() {
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..15 {
        b.insert(rng.gen_range(0, 38));
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..12 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut rng = thread_rng();
        let mut newb = data.lock().unwrap();
        for y in (8*x)..(8*(x+1)) {
            newb.insert(rng.gen_range(0, 38));
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn bst_insertion_random_medium_seq() {
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        b.insert(rng.gen_range(0, 1500));
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(rng.gen_range(0, 1500));
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn bst_insertion_random_medium_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        b.insert(rng.gen_range(0, 1500));
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut rng = thread_rng();
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(rng.gen_range(0, 1500));
            }
             newvar.write(trans, cur)
            });
        }));
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    for child in children {
        let _ = child.join();
    }
}

fn bst_insertion_random_medium_single_lock() {
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        b.insert(rng.gen_range(0, 1500));
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut rng = thread_rng();
        let mut newb = data.lock().unwrap();
        newb.insert(rng.gen_range(0, 1500));
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn bst_insertion_random_large_seq() {
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..5000 {
        b.insert(rng.gen_range(0, 7500));
    }
    let mut rng = thread_rng();
    let start = PreciseTime::now();
    for x in 0..96 {
        b.insert(rng.gen_range(0, 7500));
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn bst_insertion_random_large_stm() {
    let mut children = vec![];
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..5000 {
        b.insert(rng.gen_range(0, 7500));
    }
    let var = TVar::new(b);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut rng = thread_rng();
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                cur.insert(rng.gen_range(0, 7500));
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn bst_insertion_random_large_single_lock() {
    let mut b = Bst::new();
    let mut rng = thread_rng();
    for x in 0..5000 {
        b.insert(rng.gen_range(0, 7500));
    }
    let l = Arc::new(Mutex::new(b));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut rng = thread_rng();
        let mut newb = data.lock().unwrap();
        newb.insert(rng.gen_range(0, 7500));
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}


fn ll_insertion_small_with_all_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        l.insert(x + 1000);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));   
    //assert_eq!(l.len(), 25);
}

#[test]
fn ll_insertion_small_with_all_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..16 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (6*x)..(6*(x+1)) {
                 cur.insert(y + 1000);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 25);
}


fn ll_insertion_small_with_all_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..1000 {
        ll.insert(x);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..16 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (6*x)..(6*(x+1)) {
            newb.insert(y + 1000);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 25);
}


fn ll_insertion_medium_with_all_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        l.insert(x + 1000);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 85);
}

fn ll_insertion_medium_with_all_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(y + 1000);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 85);
}

fn ll_insertion_medium_with_all_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..1000 {
        ll.insert(x);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        newb.insert(x + 1000);
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 85);
}

fn ll_insertion_large_with_all_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..3000 {
        l.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        l.insert(x + 3000);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 125);
}

fn ll_insertion_large_with_all_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..3000 {
        l.insert(x);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(y + 115);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 125);
}

fn ll_insertion_large_with_all_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..3000 {
        ll.insert(x);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        newb.insert(x + 115);
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 125);
}

fn ll_insertion_small_with_half_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        let y = if x < 48 {x - 48} else {x + 1000};
        l.insert(y);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 25);
}

fn ll_insertion_small_with_half_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..16 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (6*x)..(6*(x+1)) {
                let z = if y < 48 {y - 48} else {y + 1000};
                cur.insert(z);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 25);
}

fn ll_insertion_small_with_half_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..1000 {
        ll.insert(x);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..16 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (6*x)..(6*(x+1)) {
            let z = if y < 48 {y - 48} else {y + 1000};
            newb.insert(z);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 25);
}

fn ll_insertion_medium_with_half_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        let y = if x < 5 {x - 5} else {x + 1000};
        l.insert(y);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 85);
}

fn ll_insertion_medium_with_half_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 let z = if y < 48 {y - 48} else {y + 1000};
                cur.insert(z);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 85);
}

fn ll_insertion_medium_with_half_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..1000 {
        ll.insert(x);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        let y = if x < 5 {x - 5} else {x + 1000};
        newb.insert(y);
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 85);
}

fn ll_insertion_large_with_half_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..3000 {
        l.insert(x);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        let y = if x < 48 {x - 48} else {x + 115};
        l.insert(y);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 125);
}

fn ll_insertion_large_with_half_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..3000 {
        l.insert(x);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 let z = if y < 48 {x - 48} else {y + 115};
                cur.insert(z);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 125);
}

fn ll_insertion_large_with_half_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..3000 {
        ll.insert(x);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        let y = if x < 5 {x - 5} else {x + 115};
        newb.insert(y);
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 125);
}

fn ll_insertion_small_with_no_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x * 2);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        l.insert(x * 2  + 1);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 25);
}

fn ll_insertion_small_with_no_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x * 2);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..16 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (6*x)..(6*(x+1)) {
                 cur.insert(y * 2  + 1);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 25);
}

fn ll_insertion_small_with_no_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..1000 {
        ll.insert(x * 2);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..8 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        for y in (12*x)..(12*(x+1)) {
            newb.insert(x * 2  + 1);
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 25);
}

fn ll_insertion_medium_with_no_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x * 2);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        l.insert(x * 2  + 1);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 85);
}

fn ll_insertion_medium_with_no_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..1000 {
        l.insert(x * 2);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(y * 2  + 1);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 85);
}

fn ll_insertion_medium_with_no_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..1000 {
        ll.insert(x * 2);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        newb.insert(x * 2  + 1);
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 85);
}

fn ll_insertion_large_with_no_collisions_seq() {
    let mut l = Ll::new();
    for x in 0..3000 {
        l.insert(x * 2);
    }
    let start = PreciseTime::now();
    for x in 0..96 {
        l.insert(x * 2  + 1);
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(l.len(), 125);
}

fn ll_insertion_large_with_no_collisions_stm() {
    let mut children = vec![];
    let mut l = Ll::new();
    for x in 0..3000 {
        l.insert(x * 2);
    }
    let var = TVar::new(l);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(y * 2  + 1);
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    //assert_eq!(var.read_atomic().len(), 125);
}

fn ll_insertion_large_with_no_collisions_single_lock() {
    let mut ll = Ll::new();
    for x in 0..3000 {
        ll.insert(x * 2);
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut newb = data.lock().unwrap();
        newb.insert(x * 2  + 1);
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
    let mut newl = l.lock().unwrap();
    //assert_eq!(newl.len(), 125);
}

fn ll_insertion_random_small_seq() {
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let start = PreciseTime::now();
    let mut rng = thread_rng();
    for x in 0..96 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_small_stm() {
    let mut children = vec![];
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let var = TVar::new(ll);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut rng = thread_rng();
             let mut cur = try!(newvar.read(trans));
             for _ in (6*x)..(6*(x+1)) {
                 cur.insert(rng.gen_range(0, 1500));
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_small_single_lock() {
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..16 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut rng = thread_rng();
        let mut newb = data.lock().unwrap();
        for _ in (6*x)..(6*(x+1)) {
            newb.insert(rng.gen_range(0, 1500));
        }
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_medium_seq() {
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let mut rng = thread_rng();
    let start = PreciseTime::now();
    for x in 0..96 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_medium_stm() {
    let mut children = vec![];
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let var = TVar::new(ll);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut rng = thread_rng();
             let mut cur = try!(newvar.read(trans));
             for _ in (8*x)..(8*(x+1)) {
                 cur.insert(rng.gen_range(0, 1500));
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_medium_single_lock() {
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..1000 {
        ll.insert(rng.gen_range(0, 1500));
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut rng = thread_rng();
        let mut newb = data.lock().unwrap();
        newb.insert(rng.gen_range(0, 1500));
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_large_seq() {
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..3000 {
        ll.insert(rng.gen_range(0, 4500));
    }
    let mut rng = thread_rng();
    let start = PreciseTime::now();
    for x in 0..96 {
        ll.insert(rng.gen_range(0, 4500));
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_large_stm() {
    let mut children = vec![];
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..3000 {
        ll.insert(rng.gen_range(0, 4500));
    }
    let var = TVar::new(ll);
    let start = PreciseTime::now();
    for x in 0..12 {
        let newvar = var.clone();
        children.push(thread::spawn(move || {
            atomically(|trans| {
             let mut rng = thread_rng();
             let mut cur = try!(newvar.read(trans));
             for y in (8*x)..(8*(x+1)) {
                 cur.insert(rng.gen_range(0, 4500));
            }
             newvar.write(trans, cur)
            });
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}

fn ll_insertion_random_large_single_lock() {
    let mut ll = Ll::new();
    let mut rng = thread_rng();
    for x in 0..3000 {
        ll.insert(rng.gen_range(0, 4500));
    }
    let l = Arc::new(Mutex::new(ll));
    let mut children = vec![];
    let start = PreciseTime::now();
    for x in 0..96 {
        let data = l.clone();
        children.push(thread::spawn(move || {
        let mut rng = thread_rng();
        let mut newb = data.lock().unwrap();
        newb.insert(rng.gen_range(0, 4500));
        }));
    }
    for child in children {
        let _ = child.join();
    }
    let end = PreciseTime::now();
    println!("{} seconds for whatever you did.", start.to(end));
}