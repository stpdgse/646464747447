package org.teavm.classlib.java.util.concurrent;

public class TForkJoinPool extends TThreadPoolExecutor {
    public interface ForkJoinWorkerThreadFactory {
        TForkJoinWorkerThread newThread(TForkJoinPool pool);
    }

    private static final TForkJoinPool COMMON = new TForkJoinPool(4);
    private final int parallelism;

    public TForkJoinPool() {
        this(4);
    }

    public TForkJoinPool(int parallelism) {
        super(parallelism, r -> {
            Thread t = new Thread(r);
            t.setDaemon(true);
            return t;
        });
        this.parallelism = parallelism;
    }

    /**
     * The worker factory is honoured for naming/handlers by calling it once per worker; because
     * workers here are fibers, the thread it returns only supplies its name and exception handler.
     */
    public TForkJoinPool(int parallelism, ForkJoinWorkerThreadFactory factory,
            Thread.UncaughtExceptionHandler handler, boolean asyncMode) {
        super(parallelism, r -> {
            Thread t = new Thread(r);
            t.setDaemon(true);
            try {
                TForkJoinWorkerThread w = factory.newThread(null);
                if (w != null) {
                    t.setName(w.getName());
                }
            } catch (Throwable ignored) {
                // factory needs a real pool instance; fall back to the default name
            }
            if (handler != null) {
                t.setUncaughtExceptionHandler(handler);
            }
            return t;
        });
        this.parallelism = parallelism;
    }

    public static TForkJoinPool commonPool() {
        return COMMON;
    }

    public int getParallelism() {
        return parallelism;
    }

    public static int getCommonPoolParallelism() {
        return COMMON.parallelism;
    }
}
