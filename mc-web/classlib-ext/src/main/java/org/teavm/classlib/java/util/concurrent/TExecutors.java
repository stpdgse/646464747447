package org.teavm.classlib.java.util.concurrent;

public final class TExecutors {
    private TExecutors() {
    }

    public static TExecutorService newFixedThreadPool(int n) {
        return new TThreadPoolExecutor(n, null);
    }

    public static TExecutorService newFixedThreadPool(int n, TThreadFactory f) {
        return new TThreadPoolExecutor(n, f);
    }

    public static TExecutorService newCachedThreadPool() {
        return new TThreadPoolExecutor(Integer.MAX_VALUE, null);
    }

    public static TExecutorService newCachedThreadPool(TThreadFactory f) {
        return new TThreadPoolExecutor(Integer.MAX_VALUE, f);
    }

    public static TExecutorService newSingleThreadExecutor() {
        return new TThreadPoolExecutor(1, null);
    }

    public static TExecutorService newSingleThreadExecutor(TThreadFactory f) {
        return new TThreadPoolExecutor(1, f);
    }

    public static TExecutorService newWorkStealingPool(int n) {
        return new TThreadPoolExecutor(n, null);
    }

    public static TExecutorService newVirtualThreadPerTaskExecutor() {
        return new TThreadPoolExecutor(Integer.MAX_VALUE, null);
    }

    public static TScheduledExecutorService newScheduledThreadPool(int n) {
        return new TScheduledThreadPoolExecutor(n);
    }

    public static TScheduledExecutorService newScheduledThreadPool(int n, TThreadFactory f) {
        return new TScheduledThreadPoolExecutor(n, f);
    }

    public static TScheduledExecutorService newSingleThreadScheduledExecutor() {
        return new TScheduledThreadPoolExecutor(1);
    }

    public static TScheduledExecutorService newSingleThreadScheduledExecutor(TThreadFactory f) {
        return new TScheduledThreadPoolExecutor(1, f);
    }

    public static TThreadFactory defaultThreadFactory() {
        return Thread::new;
    }
}
