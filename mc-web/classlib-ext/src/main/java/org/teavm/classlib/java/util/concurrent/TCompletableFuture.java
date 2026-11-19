package org.teavm.classlib.java.util.concurrent;

import java.util.ArrayList;
import java.util.List;
import java.util.function.BiConsumer;
import java.util.function.BiFunction;
import java.util.function.Consumer;
import java.util.function.Function;
import java.util.function.Supplier;
import org.teavm.classlib.java.lang.TObject;
import org.teavm.classlib.java.lang.TRunnable;

/** CompletableFuture for fiber-based threads. Async variants hand work to the supplied executor. */
public class TCompletableFuture<T> extends TObject implements TFuture<T>, TCompletionStage<T> {
    private static final class Failure {
        final Throwable error;

        Failure(Throwable error) {
            this.error = error;
        }
    }

    private Object value;
    private boolean done;
    private boolean cancelled;
    private List<Runnable> callbacks;

    public TCompletableFuture() {
    }

    private static TExecutor defaultExecutor() {
        return TForkJoinPool.commonPool();
    }

    private static void runOn(TExecutor e, Runnable r) {
        TRunnable t = r::run;
        e.execute(t);
    }

    // creation ---------------------------------------------------------------------------------

    public static <U> TCompletableFuture<U> completedFuture(U v) {
        TCompletableFuture<U> f = new TCompletableFuture<>();
        f.complete(v);
        return f;
    }

    public static <U> TCompletableFuture<U> failedFuture(Throwable ex) {
        TCompletableFuture<U> f = new TCompletableFuture<>();
        f.completeExceptionally(ex);
        return f;
    }

    public static <U> TCompletableFuture<U> supplyAsync(Supplier<U> s) {
        return supplyAsync(s, defaultExecutor());
    }

    public static <U> TCompletableFuture<U> supplyAsync(Supplier<U> s, TExecutor e) {
        TCompletableFuture<U> f = new TCompletableFuture<>();
        runOn(e, () -> {
            try {
                f.complete(s.get());
            } catch (Throwable ex) {
                f.completeExceptionally(ex);
            }
        });
        return f;
    }

    public static TCompletableFuture<Void> runAsync(Runnable r) {
        return runAsync(r, defaultExecutor());
    }

    public static TCompletableFuture<Void> runAsync(Runnable r, TExecutor e) {
        TCompletableFuture<Void> f = new TCompletableFuture<>();
        runOn(e, () -> {
            try {
                r.run();
                f.complete(null);
            } catch (Throwable ex) {
                f.completeExceptionally(ex);
            }
        });
        return f;
    }

    public static TCompletableFuture<Void> allOf(TCompletableFuture<?>... fs) {
        TCompletableFuture<Void> result = new TCompletableFuture<>();
        if (fs.length == 0) {
            result.complete(null);
            return result;
        }
        int[] remaining = {fs.length};
        for (TCompletableFuture<?> f : fs) {
            f.onDone(() -> {
                boolean last;
                synchronized (remaining) {
                    last = --remaining[0] == 0;
                }
                if (last) {
                    Throwable err = null;
                    for (TCompletableFuture<?> g : fs) {
                        if (g.value instanceof Failure) {
                            err = ((Failure) g.value).error;
                            break;
                        }
                    }
                    if (err != null) {
                        result.completeExceptionally(wrap(err));
                    } else {
                        result.complete(null);
                    }
                }
            });
        }
        return result;
    }

    public static TCompletableFuture<Object> anyOf(TCompletableFuture<?>... fs) {
        TCompletableFuture<Object> result = new TCompletableFuture<>();
        for (TCompletableFuture<?> f : fs) {
            f.onDone(() -> {
                if (f.value instanceof Failure) {
                    result.completeExceptionally(wrap(((Failure) f.value).error));
                } else {
                    result.complete(f.value);
                }
            });
        }
        return result;
    }

    // completion -------------------------------------------------------------------------------

    private boolean finish(Object v) {
        List<Runnable> cbs;
        synchronized (this) {
            if (done) {
                return false;
            }
            value = v;
            done = true;
            cbs = callbacks;
            callbacks = null;
            notifyAll();
        }
        if (cbs != null) {
            for (Runnable r : cbs) {
                r.run();
            }
        }
        return true;
    }

    public boolean complete(T v) {
        return finish(v);
    }

    public boolean completeExceptionally(Throwable ex) {
        if (ex == null) {
            throw new NullPointerException();
        }
        return finish(new Failure(ex));
    }

    private void onDone(Runnable r) {
        synchronized (this) {
            if (!done) {
                if (callbacks == null) {
                    callbacks = new ArrayList<>();
                }
                callbacks.add(r);
                return;
            }
        }
        r.run();
    }

    private static Throwable wrap(Throwable t) {
        return t instanceof java.util.concurrent.CompletionException ? t
                : new java.util.concurrent.CompletionException(t);
    }

    private static Throwable unwrap(Throwable t) {
        return t instanceof java.util.concurrent.CompletionException && t.getCause() != null ? t.getCause() : t;
    }

    // chaining ---------------------------------------------------------------------------------

    private <U> TCompletableFuture<U> chain(TExecutor exec, Function<Object, Object> body) {
        TCompletableFuture<U> out = new TCompletableFuture<>();
        onDone(() -> {
            Runnable job = () -> {
                try {
                    @SuppressWarnings("unchecked") U r = (U) body.apply(value);
                    out.complete(r);
                } catch (Throwable ex) {
                    out.completeExceptionally(wrap(ex));
                }
            };
            if (exec == null) {
                job.run();
            } else {
                runOn(exec, job);
            }
        });
        return out;
    }

    private Object failureOrNull() {
        return value instanceof Failure ? value : null;
    }

    private static Object throwIfFailed(Object v) {
        if (v instanceof Failure) {
            throw sneaky(wrap(((Failure) v).error));
        }
        return v;
    }

    @SuppressWarnings("unchecked")
    private static <E extends Throwable> RuntimeException sneaky(Throwable t) throws E {
        throw (E) t;
    }

    public <U> TCompletableFuture<U> thenApply(Function<? super T, ? extends U> fn) {
        return thenApplyAsync0(fn, null);
    }

    public <U> TCompletableFuture<U> thenApplyAsync(Function<? super T, ? extends U> fn, TExecutor e) {
        return thenApplyAsync0(fn, e);
    }

    @SuppressWarnings("unchecked")
    private <U> TCompletableFuture<U> thenApplyAsync0(Function<? super T, ? extends U> fn, TExecutor e) {
        return chain(e, v -> fn.apply((T) throwIfFailed(v)));
    }

    public TCompletableFuture<Void> thenAccept(Consumer<? super T> c) {
        return thenAcceptAsync0(c, null);
    }

    public TCompletableFuture<Void> thenAcceptAsync(Consumer<? super T> c, TExecutor e) {
        return thenAcceptAsync0(c, e);
    }

    @SuppressWarnings("unchecked")
    private TCompletableFuture<Void> thenAcceptAsync0(Consumer<? super T> c, TExecutor e) {
        return chain(e, v -> {
            c.accept((T) throwIfFailed(v));
            return null;
        });
    }

    public TCompletableFuture<Void> thenRun(Runnable r) {
        return thenRunAsync0(r, null);
    }

    public TCompletableFuture<Void> thenRunAsync(Runnable r, TExecutor e) {
        return thenRunAsync0(r, e);
    }

    private TCompletableFuture<Void> thenRunAsync0(Runnable r, TExecutor e) {
        return chain(e, v -> {
            throwIfFailed(v);
            r.run();
            return null;
        });
    }

    public <U> TCompletableFuture<U> thenCompose(Function<? super T, ? extends TCompletionStage<U>> fn) {
        return thenComposeAsync0(fn, null);
    }

    public <U> TCompletableFuture<U> thenComposeAsync(Function<? super T, ? extends TCompletionStage<U>> fn,
            TExecutor e) {
        return thenComposeAsync0(fn, e);
    }

    @SuppressWarnings("unchecked")
    private <U> TCompletableFuture<U> thenComposeAsync0(Function<? super T, ? extends TCompletionStage<U>> fn,
            TExecutor e) {
        TCompletableFuture<U> out = new TCompletableFuture<>();
        onDone(() -> {
            Runnable job = () -> {
                try {
                    TCompletableFuture<U> inner = fn.apply((T) throwIfFailed(value)).toCompletableFuture();
                    inner.onDone(() -> {
                        if (inner.value instanceof Failure) {
                            out.completeExceptionally(wrap(((Failure) inner.value).error));
                        } else {
                            out.complete((U) inner.value);
                        }
                    });
                } catch (Throwable ex) {
                    out.completeExceptionally(wrap(ex));
                }
            };
            if (e == null) {
                job.run();
            } else {
                runOn(e, job);
            }
        });
        return out;
    }

    public <U, V> TCompletableFuture<V> thenCombine(TCompletionStage<? extends U> other,
            BiFunction<? super T, ? super U, ? extends V> fn) {
        return thenCombineAsync0(other, fn, null);
    }

    public <U, V> TCompletableFuture<V> thenCombineAsync(TCompletionStage<? extends U> other,
            BiFunction<? super T, ? super U, ? extends V> fn, TExecutor e) {
        return thenCombineAsync0(other, fn, e);
    }

    @SuppressWarnings("unchecked")
    private <U, V> TCompletableFuture<V> thenCombineAsync0(TCompletionStage<? extends U> other,
            BiFunction<? super T, ? super U, ? extends V> fn, TExecutor e) {
        TCompletableFuture<V> out = new TCompletableFuture<>();
        TCompletableFuture<? extends U> o = other.toCompletableFuture();
        onDone(() -> o.onDone(() -> {
            Runnable job = () -> {
                try {
                    out.complete(fn.apply((T) throwIfFailed(value), (U) throwIfFailed(o.value)));
                } catch (Throwable ex) {
                    out.completeExceptionally(wrap(ex));
                }
            };
            if (e == null) {
                job.run();
            } else {
                runOn(e, job);
            }
        }));
        return out;
    }

    @SuppressWarnings("unchecked")
    public <U> TCompletableFuture<U> applyToEither(TCompletionStage<? extends T> other,
            Function<? super T, U> fn) {
        TCompletableFuture<U> out = new TCompletableFuture<>();
        Runnable[] fire = new Runnable[1];
        for (TCompletableFuture<? extends T> src : new TCompletableFuture[] {this, other.toCompletableFuture()}) {
            src.onDone(() -> {
                try {
                    out.complete(fn.apply((T) throwIfFailed(src.value)));
                } catch (Throwable ex) {
                    out.completeExceptionally(wrap(ex));
                }
            });
        }
        return out;
    }

    public TCompletableFuture<T> exceptionally(Function<Throwable, ? extends T> fn) {
        return exceptionallyAsync0(fn, null);
    }

    public TCompletableFuture<T> exceptionallyAsync(Function<Throwable, ? extends T> fn, TExecutor e) {
        return exceptionallyAsync0(fn, e);
    }

    @SuppressWarnings("unchecked")
    private TCompletableFuture<T> exceptionallyAsync0(Function<Throwable, ? extends T> fn, TExecutor e) {
        return chain(e, v -> v instanceof Failure ? fn.apply(((Failure) v).error) : v);
    }

    public TCompletableFuture<T> exceptionallyComposeAsync(
            Function<Throwable, ? extends TCompletionStage<T>> fn, TExecutor e) {
        TCompletableFuture<T> out = new TCompletableFuture<>();
        onDone(() -> {
            if (!(value instanceof Failure)) {
                @SuppressWarnings("unchecked") T v = (T) value;
                out.complete(v);
                return;
            }
            runOn(e, () -> {
                try {
                    TCompletableFuture<T> inner = fn.apply(((Failure) value).error).toCompletableFuture();
                    inner.onDone(() -> {
                        if (inner.value instanceof Failure) {
                            out.completeExceptionally(wrap(((Failure) inner.value).error));
                        } else {
                            @SuppressWarnings("unchecked") T v = (T) inner.value;
                            out.complete(v);
                        }
                    });
                } catch (Throwable ex) {
                    out.completeExceptionally(wrap(ex));
                }
            });
        });
        return out;
    }

    public <U> TCompletableFuture<U> handle(BiFunction<? super T, Throwable, ? extends U> fn) {
        return handleAsync0(fn, null);
    }

    public <U> TCompletableFuture<U> handleAsync(BiFunction<? super T, Throwable, ? extends U> fn, TExecutor e) {
        return handleAsync0(fn, e);
    }

    @SuppressWarnings("unchecked")
    private <U> TCompletableFuture<U> handleAsync0(BiFunction<? super T, Throwable, ? extends U> fn,
            TExecutor e) {
        return chain(e, v -> v instanceof Failure
                ? fn.apply(null, ((Failure) v).error) : fn.apply((T) v, null));
    }

    public TCompletableFuture<T> whenComplete(BiConsumer<? super T, ? super Throwable> action) {
        return whenCompleteAsync0(action, null);
    }

    public TCompletableFuture<T> whenCompleteAsync(BiConsumer<? super T, ? super Throwable> action, TExecutor e) {
        return whenCompleteAsync0(action, e);
    }

    @SuppressWarnings("unchecked")
    private TCompletableFuture<T> whenCompleteAsync0(BiConsumer<? super T, ? super Throwable> action,
            TExecutor e) {
        TCompletableFuture<T> out = new TCompletableFuture<>();
        onDone(() -> {
            Runnable job = () -> {
                Throwable err = value instanceof Failure ? ((Failure) value).error : null;
                try {
                    action.accept(err == null ? (T) value : null, err);
                } catch (Throwable ex) {
                    if (err == null) {
                        out.completeExceptionally(wrap(ex));
                        return;
                    }
                }
                if (err != null) {
                    out.completeExceptionally(wrap(err));
                } else {
                    out.complete((T) value);
                }
            };
            if (e == null) {
                job.run();
            } else {
                runOn(e, job);
            }
        });
        return out;
    }

    // results ----------------------------------------------------------------------------------

    @Override
    public TCompletableFuture<T> toCompletableFuture() {
        return this;
    }

    @Override
    public synchronized boolean isDone() {
        return done;
    }

    @Override
    public synchronized boolean isCancelled() {
        return cancelled;
    }

    public synchronized boolean isCompletedExceptionally() {
        return done && value instanceof Failure;
    }

    @Override
    public boolean cancel(boolean mayInterrupt) {
        boolean ok = finish(new Failure(new java.util.concurrent.CancellationException()));
        if (ok) {
            cancelled = true;
        }
        return ok || cancelled;
    }

    @SuppressWarnings("unchecked")
    public T getNow(T valueIfAbsent) {
        synchronized (this) {
            if (!done) {
                return valueIfAbsent;
            }
        }
        return (T) throwIfFailed(value);
    }

    @SuppressWarnings("unchecked")
    public T join() {
        synchronized (this) {
            boolean interrupted = false;
            while (!done) {
                try {
                    wait();
                } catch (InterruptedException e) {
                    interrupted = true;
                }
            }
            if (interrupted) {
                Thread.currentThread().interrupt();
            }
        }
        if (value instanceof Failure) {
            Throwable err = ((Failure) value).error;
            if (err instanceof java.util.concurrent.CancellationException) {
                throw (java.util.concurrent.CancellationException) err;
            }
            throw (RuntimeException) wrap(err);
        }
        return (T) value;
    }

    @Override
    @SuppressWarnings("unchecked")
    public T get() throws InterruptedException, TExecutionException {
        synchronized (this) {
            while (!done) {
                wait();
            }
        }
        return report();
    }

    @Override
    public T get(long timeout, TTimeUnit unit) throws InterruptedException, TExecutionException, TTimeoutException {
        long deadline = System.currentTimeMillis() + unit.toMillis(timeout);
        synchronized (this) {
            while (!done) {
                long left = deadline - System.currentTimeMillis();
                if (left <= 0) {
                    throw new TTimeoutException();
                }
                wait(left);
            }
        }
        return report();
    }

    @SuppressWarnings("unchecked")
    private T report() throws TExecutionException {
        if (value instanceof Failure) {
            Throwable err = unwrap(((Failure) value).error);
            if (err instanceof java.util.concurrent.CancellationException) {
                throw new TCancellationException();
            }
            throw new TExecutionException(err);
        }
        return (T) value;
    }
}
