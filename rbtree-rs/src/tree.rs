struct Node<T> {
    val: *mut T,
    left: *mut Node<T>,
    right: *mut Node<T>,
}

pub struct RBTree {}
