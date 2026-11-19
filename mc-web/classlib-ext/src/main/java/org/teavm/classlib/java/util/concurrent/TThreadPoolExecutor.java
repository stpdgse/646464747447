package org.teavm.classlib.java.util.concurrent;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.List;
import org.teavm.classlib.java.lang.TRunnable;

/**
 * Fiber based pool. TeaVM threads are cooperative fibers, so "threads" here run
 * interleaved on the one underlying JS/Worker thread.
 */
public class TThreadPoolExecutor extends TAbstractExecutorService {
    private final int maxThreads;
    private final TThreadFactory factory;
    private final ArrayDeque<TRunnable> queue = new ArrayDeque<>();
    private int idle;
    private int workers;
    private boolean shutdown;

    public TThreadPoolExecutor(int maxThreads, TThreadFactory factory) {
        this.maxThreads = Math.max(1, maxThreads);
        this.factory = factory != null ? factory : Thread::new;
    }

    @Override
    public void execute(TRunnable command) {
        if (command == null) {
            throw new NullPointerException();
        }
        synchronized (this) {
            if (shutdown) {
                throw new TRejectedExecutionException("Executor has been shut down");
            }
            queue.add(command);
            if (idle > 0) {
                notify();
                return;
            }
            if (workers >= maxThreads) {
                return;
            }
            workers++;
        }
        Thread t = factory.newThread(this::workerLoop);
        t.start();
    }

    private void workerLoop() {
        while (true) {
            TRunnable task;
            synchronized (this) {
                while (queue.isEmpty()) {
                    if (shutdown) {
                        workers--;
                        notifyAll();
                        return;
                    }
                    idle++;
                    try {
                        wait();
                    } catch (InterruptedException e) {
                        // continue
                    } finally {
                        idle--;
                    }
                }
                task = queue.poll();
            }
            try {
                task.run();
            } catch (Throwable e) {
                Thread t = Thread.currentThread();
                Thread.UncaughtExceptionHandler h = t.getUncaughtExceptionHandler();
                if (h != null) {
                    h.uncaughtException(t, e);
                }
            }
        }
    }

    @Override
    public synchronized void shutdown() {
        shutdown = true;
        notifyAll();
    }

    @Override
    public synchronized List<Runnable> shutdownNow() {
        shutdown = true;
        List<Runnable> rest = new ArrayList<>();
        for (TRunnable r : queue) {
            rest.add(r::run);
        }
        queue.clear();
        notifyAll();
        return rest;
    }

    @Override
    public synchronized boolean isShutdown() {
        return shutdown;
    }

    @Override
    public synchronized boolean isTerminated() {
        return shutdown && workers == 0 && queue.isEmpty();
    }

    @Override
    public synchronized boolean awaitTermination(long timeout, TTimeUnit unit) throws InterruptedException {
        long deadline = System.currentTimeMillis() + unit.toMillis(timeout);
        while (!(shutdown && workers == 0 && queue.isEmpty())) {
            long left = deadline - System.currentTimeMillis();
            if (left <= 0) {
                return false;
            }
            wait(left);
        }
        return true;
    }
}
