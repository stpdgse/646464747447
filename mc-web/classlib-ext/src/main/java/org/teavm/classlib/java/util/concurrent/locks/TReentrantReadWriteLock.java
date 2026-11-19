package org.teavm.classlib.java.util.concurrent.locks;

import org.teavm.classlib.java.lang.TObject;
import org.teavm.classlib.java.util.concurrent.TTimeUnit;

public class TReentrantReadWriteLock extends TObject implements TReadWriteLock {
    private final State state = new State();
    private final ReadLock readLock = new ReadLock(state);
    private final WriteLock writeLock = new WriteLock(state);

    public TReentrantReadWriteLock() {
    }

    public TReentrantReadWriteLock(boolean fair) {
    }

    @Override
    public ReadLock readLock() {
        return readLock;
    }

    @Override
    public WriteLock writeLock() {
        return writeLock;
    }

    public int getReadLockCount() {
        return state.readers;
    }

    public boolean isWriteLocked() {
        return state.writer != null;
    }

    public boolean isWriteLockedByCurrentThread() {
        return state.writer == Thread.currentThread();
    }

    static final class State {
        Thread writer;
        int writeHolds;
        int readers;
    }

    public static class ReadLock extends TObject implements TLock {
        private final State state;

        ReadLock(State state) {
            this.state = state;
        }

        @Override
        public void lock() {
            synchronized (state) {
                while (state.writer != null && state.writer != Thread.currentThread()) {
                    try {
                        state.wait();
                    } catch (InterruptedException e) {
                        // uninterruptible
                    }
                }
                state.readers++;
            }
        }

        @Override
        public void lockInterruptibly() throws InterruptedException {
            synchronized (state) {
                while (state.writer != null && state.writer != Thread.currentThread()) {
                    state.wait();
                }
                state.readers++;
            }
        }

        @Override
        public boolean tryLock() {
            synchronized (state) {
                if (state.writer != null && state.writer != Thread.currentThread()) {
                    return false;
                }
                state.readers++;
                return true;
            }
        }

        @Override
        public boolean tryLock(long time, TTimeUnit unit) throws InterruptedException {
            long deadline = System.currentTimeMillis() + unit.toMillis(time);
            synchronized (state) {
                while (state.writer != null && state.writer != Thread.currentThread()) {
                    long left = deadline - System.currentTimeMillis();
                    if (left <= 0) {
                        return false;
                    }
                    state.wait(left);
                }
                state.readers++;
                return true;
            }
        }

        @Override
        public void unlock() {
            synchronized (state) {
                if (state.readers <= 0) {
                    throw new IllegalMonitorStateException();
                }
                if (--state.readers == 0) {
                    state.notifyAll();
                }
            }
        }

        @Override
        public TCondition newCondition() {
            throw new UnsupportedOperationException();
        }
    }

    public static class WriteLock extends TObject implements TLock {
        private final State state;

        WriteLock(State state) {
            this.state = state;
        }

        private boolean blocked(Thread current) {
            return (state.writer != null && state.writer != current)
                    || (state.writer == null && state.readers > 0);
        }

        @Override
        public void lock() {
            Thread current = Thread.currentThread();
            synchronized (state) {
                while (blocked(current)) {
                    try {
                        state.wait();
                    } catch (InterruptedException e) {
                        // uninterruptible
                    }
                }
                state.writer = current;
                state.writeHolds++;
            }
        }

        @Override
        public void lockInterruptibly() throws InterruptedException {
            Thread current = Thread.currentThread();
            synchronized (state) {
                while (blocked(current)) {
                    state.wait();
                }
                state.writer = current;
                state.writeHolds++;
            }
        }

        @Override
        public boolean tryLock() {
            Thread current = Thread.currentThread();
            synchronized (state) {
                if (blocked(current)) {
                    return false;
                }
                state.writer = current;
                state.writeHolds++;
                return true;
            }
        }

        @Override
        public boolean tryLock(long time, TTimeUnit unit) throws InterruptedException {
            long deadline = System.currentTimeMillis() + unit.toMillis(time);
            Thread current = Thread.currentThread();
            synchronized (state) {
                while (blocked(current)) {
                    long left = deadline - System.currentTimeMillis();
                    if (left <= 0) {
                        return false;
                    }
                    state.wait(left);
                }
                state.writer = current;
                state.writeHolds++;
                return true;
            }
        }

        @Override
        public void unlock() {
            synchronized (state) {
                if (state.writer != Thread.currentThread()) {
                    throw new IllegalMonitorStateException();
                }
                if (--state.writeHolds == 0) {
                    state.writer = null;
                    state.notifyAll();
                }
            }
        }

        @Override
        public TCondition newCondition() {
            throw new UnsupportedOperationException();
        }
    }
}
