package org.teavm.classlib.java.util.concurrent;

import java.util.Collection;
import java.util.List;

public interface TExecutorService extends TExecutor, AutoCloseable {
    void shutdown();
    List<Runnable> shutdownNow();
    boolean isShutdown();
    boolean isTerminated();
    boolean awaitTermination(long timeout, TTimeUnit unit) throws InterruptedException;
    <T> TFuture<T> submit(@SuppressWarnings("rawtypes") TCallable task);
    <T> TFuture<T> submit(Runnable task, T result);
    TFuture<?> submit(Runnable task);
    <T> List<TFuture<T>> invokeAll(@SuppressWarnings("rawtypes") Collection<? extends TCallable> tasks) throws InterruptedException;

    @Override
    default void close() {
        shutdown();
        try {
            awaitTermination(1, TTimeUnit.DAYS);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
        }
    }
}
