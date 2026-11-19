package org.teavm.classlib.java.util.concurrent;

public class TForkJoinWorkerThread extends Thread {
    private final TForkJoinPool pool;

    protected TForkJoinWorkerThread(TForkJoinPool pool) {
        this.pool = pool;
    }

    public TForkJoinPool getPool() {
        return pool;
    }

    protected void onStart() {
    }

    protected void onTermination(Throwable exception) {
    }
}
