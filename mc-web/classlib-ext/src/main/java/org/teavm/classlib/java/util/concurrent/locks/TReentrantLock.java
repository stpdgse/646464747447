package org.teavm.classlib.java.util.concurrent.locks;

import org.teavm.classlib.java.lang.TObject;
import org.teavm.classlib.java.util.concurrent.TTimeUnit;

/**
 * Reentrant lock for TeaVM fibers. Threads are cooperative, so contention only
 * happens when the owner parks (sleep, wait, I/O) while holding the lock.
 */
public class TReentrantLock extends TObject implements TLock {
    private Thread owner;
    private int holds;
    private final boolean fair;

    public TReentrantLock() {
        this(false);
    }

    public TReentrantLock(boolean fair) {
        this.fair = fair;
    }

    @Override
    public void lock() {
        Thread current = Thread.currentThread();
        synchronized (this) {
            while (owner != null && owner != current) {
                try {
                    wait();
                } catch (InterruptedException e) {
                    // lock() is uninterruptible: keep waiting
                }
            }
            owner = current;
            holds++;
        }
    }

    @Override
    public void lockInterruptibly() throws InterruptedException {
        Thread current = Thread.currentThread();
        synchronized (this) {
            while (owner != null && owner != current) {
                wait();
            }
            owner = current;
            holds++;
        }
    }

    @Override
    public boolean tryLock() {
        Thread current = Thread.currentThread();
        synchronized (this) {
            if (owner != null && owner != current) {
                return false;
            }
            owner = current;
            holds++;
            return true;
        }
    }

    @Override
    public boolean tryLock(long time, TTimeUnit unit) throws InterruptedException {
        long deadline = System.currentTimeMillis() + unit.toMillis(time);
        Thread current = Thread.currentThread();
        synchronized (this) {
            while (owner != null && owner != current) {
                long left = deadline - System.currentTimeMillis();
                if (left <= 0) {
                    return false;
                }
                wait(left);
            }
            owner = current;
            holds++;
            return true;
        }
    }

    @Override
    public void unlock() {
        synchronized (this) {
            if (owner != Thread.currentThread()) {
                throw new IllegalMonitorStateException();
            }
            if (--holds == 0) {
                owner = null;
                notifyAll();
            }
        }
    }

    @Override
    public TCondition newCondition() {
        return new Cond();
    }

    public boolean isLocked() {
        return owner != null;
    }

    public boolean isHeldByCurrentThread() {
        return owner == Thread.currentThread();
    }

    public int getHoldCount() {
        return owner == Thread.currentThread() ? holds : 0;
    }

    public boolean isFair() {
        return fair;
    }

    private final class Cond extends TObject implements TCondition {
        private int signals;

        private int releaseAll() {
            synchronized (TReentrantLock.this) {
                if (owner != Thread.currentThread()) {
                    throw new IllegalMonitorStateException();
                }
                int saved = holds;
                holds = 0;
                owner = null;
                TReentrantLock.this.notifyAll();
                return saved;
            }
        }

        private void reacquire(int saved) {
            Thread current = Thread.currentThread();
            synchronized (TReentrantLock.this) {
                while (owner != null) {
                    try {
                        TReentrantLock.this.wait();
                    } catch (InterruptedException e) {
                        // keep waiting
                    }
                }
                owner = current;
                holds = saved;
            }
        }

        @Override
        public void await() throws InterruptedException {
            int saved = releaseAll();
            try {
                synchronized (this) {
                    int seen = signals;
                    while (signals == seen) {
                        wait();
                    }
                }
            } finally {
                reacquire(saved);
            }
        }

        @Override
        public void awaitUninterruptibly() {
            int saved = releaseAll();
            try {
                synchronized (this) {
                    int seen = signals;
                    while (signals == seen) {
                        try {
                            wait();
                        } catch (InterruptedException e) {
                            // ignore
                        }
                    }
                }
            } finally {
                reacquire(saved);
            }
        }

        @Override
        public long awaitNanos(long nanos) throws InterruptedException {
            long start = System.nanoTime();
            int saved = releaseAll();
            try {
                synchronized (this) {
                    int seen = signals;
                    long left = nanos;
                    while (signals == seen && left > 0) {
                        wait(Math.max(1, left / 1_000_000));
                        left = nanos - (System.nanoTime() - start);
                    }
                }
            } finally {
                reacquire(saved);
            }
            return nanos - (System.nanoTime() - start);
        }

        @Override
        public boolean await(long time, TTimeUnit unit) throws InterruptedException {
            return awaitNanos(unit.toNanos(time)) > 0;
        }

        @Override
        public synchronized void signal() {
            signals++;
            notify();
        }

        @Override
        public synchronized void signalAll() {
            signals++;
            notifyAll();
        }
    }
}
