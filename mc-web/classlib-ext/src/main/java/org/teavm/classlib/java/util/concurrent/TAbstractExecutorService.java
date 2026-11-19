package org.teavm.classlib.java.util.concurrent;

import java.util.ArrayList;
import java.util.Collection;
import java.util.List;
import org.teavm.classlib.java.lang.TObject;

public abstract class TAbstractExecutorService extends TObject implements TExecutorService {
    @Override
    public <T> TFuture<T> submit(@SuppressWarnings("rawtypes") TCallable task) {
        TFutureTask<T> f = new TFutureTask<T>(task);
        execute(f);
        return f;
    }

    @Override
    public <T> TFuture<T> submit(Runnable task, T result) {
        TFutureTask<T> f = new TFutureTask<>(task, result);
        execute(f);
        return f;
    }

    @Override
    public TFuture<?> submit(Runnable task) {
        TFutureTask<Object> f = new TFutureTask<>(task, null);
        execute(f);
        return f;
    }

    @Override
    public <T> List<TFuture<T>> invokeAll(@SuppressWarnings("rawtypes") Collection<? extends TCallable> tasks) throws InterruptedException {
        List<TFuture<T>> futures = new ArrayList<>();
        for (@SuppressWarnings("rawtypes") TCallable t : tasks) {
            futures.add(this.<T>submit(t));
        }
        for (TFuture<T> f : futures) {
            try {
                f.get();
            } catch (TExecutionException e) {
                // reported through the future
            }
        }
        return futures;
    }
}
