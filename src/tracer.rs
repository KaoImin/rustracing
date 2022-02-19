use crate::sampler::Sampler;
use crate::span::{
    AsyncSpanReceiver, AsyncSpanSender, SpanSend, StartSpanOptions, SyncSpanReceiver,
    SyncSpanSender,
};

use beef::lean::Cow;

use std::marker::PhantomData;
use std::sync::Arc;

const DEFAULT_ASYNC_CHANNEL_SIZE: usize = 1024 * 1024;

/// Tracer.
///
/// # Examples
///
/// ```
/// use rustracing::Tracer;
/// use rustracing::sampler::AllSampler;
///
/// let (span_tx, span_rx) = crossbeam_channel::bounded(10);
/// let tracer = Tracer::with_sender(AllSampler, span_tx);
/// {
///    let _span = tracer.span("foo").start_with_state(());
/// }
/// let span = span_rx.try_recv().unwrap();
/// assert_eq!(span.operation_name(), "foo");
/// ```
#[derive(Debug)]
pub struct Tracer<S, T, Sender: SpanSend<T>> {
    sampler: Arc<S>,
    span_tx: Sender,
    _pin_t: PhantomData<T>,
}

impl<S: Sampler<T>, T> Tracer<S, T, SyncSpanSender<T>> {
    /// This constructor is mainly for backward compatibility, it has the same interface
    /// as in previous versions except the type of `SpanReceiver`.
    /// It builds an unbounded channel which may cause memory issues if there is no reader,
    /// prefer `with_sender()` alternative with a bounded one.
    pub fn new(sampler: S) -> (Self, SyncSpanReceiver<T>) {
        let (span_tx, span_rx) = crossbeam_channel::unbounded();
        (Self::with_sender(sampler, span_tx), span_rx)
    }
}

impl<S: Sampler<T>, T> Tracer<S, T, AsyncSpanSender<T>> {
    /// This constructor is mainly for backward compatibility, it has the same interface
    /// as in previous versions except the type of `SpanReceiver`.
    /// It builds an async channel with capacity of `1024 * 1024`, which may cause memory issues if there is no reader,
    /// prefer `with_sender()` alternative with a bounded one.
    pub fn new_async(sampler: S) -> (Self, AsyncSpanReceiver<T>) {
        let (span_tx, span_rx) = tokio::sync::mpsc::channel(DEFAULT_ASYNC_CHANNEL_SIZE);
        (Self::with_sender(sampler, span_tx), span_rx)
    }
}

impl<S: Sampler<T>, T, Sender: SpanSend<T>> Tracer<S, T, Sender> {
    /// Makes a new `Tracer` instance.
    pub fn with_sender(sampler: S, span_tx: Sender) -> Self {
        Tracer {
            sampler: Arc::new(sampler),
            span_tx,
            _pin_t: PhantomData,
        }
    }

    /// Returns `StartSpanOptions` for starting a span which has the name `operation_name`.
    pub fn span<N>(&self, operation_name: N) -> StartSpanOptions<S, T, Sender>
    where
        N: Into<Cow<'static, str>>,
    {
        StartSpanOptions::new(operation_name, &self.span_tx, &self.sampler)
    }
}

impl<S, T, Sender: SpanSend<T>> Tracer<S, T, Sender> {
    /// Clone with the given `sampler`.
    pub fn clone_with_sampler<U: Sampler<T>>(&self, sampler: U) -> Tracer<U, T, Sender> {
        Tracer {
            sampler: Arc::new(sampler),
            span_tx: self.span_tx.clone(),
            _pin_t: PhantomData,
        }
    }
}

impl<S, T, Sender: SpanSend<T>> Clone for Tracer<S, T, Sender> {
    fn clone(&self) -> Self {
        Tracer {
            sampler: Arc::clone(&self.sampler),
            span_tx: self.span_tx.clone(),
            _pin_t: PhantomData,
        }
    }
}
