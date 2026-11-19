package org.teavm.classlib.java.util.concurrent;

import org.teavm.classlib.java.lang.TObject;
import org.teavm.classlib.java.lang.TRunnable;

public class TFutureTask<V> extends TObject implements TRunnableFuture<V> {
    @SuppressWarnings("rawtypes")
    private TCallable callable;
    private V fixedResult;
    private boolean useFixed;
    private V result;
    private Throwable failure;
    private boolean done;
    private boolean cancelled;

    public TFutureTask(TCallable callable) {
        this.callable = callable;
    }

    public TFutureTask(Runnable runnable, V result) {
        this.callable = new TCallable() {
            @Override
            public TObject call() {
                runnable.run();
                return null;
            }
        };
        this.fixedResult = result;
        this.useFixed = true;
    }

    @Override
    public void run() {
        @SuppressWarnings("rawtypes") TCallable c;
        synchronized (this) {
            if (done) {
                return;
            }
            c = callable;
        }
        V value = null;
        Throwable error = null;
        try {
            @SuppressWarnings("unchecked") V v = useFixed ? fixedResult : (V) c.call();
            value = v;
        } catch (Throwable e) {
            error = e;
        }
        synchronized (this) {
            if (!done) {
                result = value;
                failure = error;
                done = true;
                notifyAll();
            }
        }
        done();
    }

    protected void done() {
    }

    @Override
    public synchronized boolean cancel(boolean mayInterruptIfRunning) {
        if (done) {
            return false;
        }
        cancelled = true;
        done = true;
        notifyAll();
        return true;
    }

    @Override
    public synchronized boolean isCancelled() {
        return cancelled;
    }

    @Override
    public synchronized boolean isDone() {
        return done;
    }

    @Override
    public synchronized V get() throws InterruptedException, TExecutionException {
        while (!done) {
            wait();
        }
        return report();
    }

    @Override
    public synchronized V get(long timeout, TTimeUnit unit)
            throws InterruptedException, TExecutionException, TTimeoutException {
        long deadline = System.currentTimeMillis() + unit.toMillis(timeout);
        while (!done) {
            long left = deadline - System.currentTimeMillis();
            if (left <= 0) {
                throw new TTimeoutException();
            }
            wait(left);
        }
        return report();
    }

    private V report() throws TExecutionException {
        if (cancelled) {
            throw new TCancellationException();
        }
        if (failure != null) {
            throw new TExecutionException(failure);
        }
        return result;
    }
}
