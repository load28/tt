#[cfg(test)]
thread_local! {
    static COUNTS: std::cell::RefCell<std::collections::HashMap<&'static str, usize>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

#[inline]
pub(crate) fn tick(_name: &'static str) {
    #[cfg(test)]
    COUNTS.with(|counts| *counts.borrow_mut().entry(_name).or_default() += 1);
}

#[cfg(test)]
pub(crate) fn measure<T>(
    run: impl FnOnce() -> T,
) -> std::collections::HashMap<&'static str, usize> {
    COUNTS.with(|counts| counts.borrow_mut().clear());
    let _ = run();
    COUNTS.with(|counts| std::mem::take(&mut *counts.borrow_mut()))
}
