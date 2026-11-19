package org.teavm.classlib.java.util.concurrent;

public class TForkJoinPool extends TThreadPoolExecutor {
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

    public static TForkJoinPool commonPool() {
        return COMMON;
    }

    public int getParallelism() {
        return parallelism;
    }
}
