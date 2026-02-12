// src/common/pool/clear.rs
//
// Clearable object trait

/// Trait for clearable objects
///
/// Types implementing this trait can be automatically cleared by object pool
pub trait Clear {
    /// Clear the object to its initial state
    fn clear(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestClear {
        value: usize,
    }

    impl Clear for TestClear {
        fn clear(&mut self) {
            self.value = 0;
        }
    }

    #[test]
    fn test_clear_trait() {
        let mut t = TestClear { value: 42 };
        assert_eq!(t.value, 42);
        t.clear();
        assert_eq!(t.value, 0);
    }
}
