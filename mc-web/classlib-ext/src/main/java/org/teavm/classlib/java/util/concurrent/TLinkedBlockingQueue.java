package org.teavm.classlib.java.util.concurrent;

import java.util.ArrayDeque;
import java.util.ArrayList;
import org.teavm.classlib.java.util.TAbstractQueue;
import org.teavm.classlib.java.util.TCollection;
import org.teavm.classlib.java.util.TIterator;

public class TLinkedBlockingQueue<E> extends TAbstractQueue<E> implements TBlockingQueue<E> {
    private final ArrayDeque<E> items = new ArrayDeque<>();
    private final int capacity;

    public TLinkedBlockingQueue() {
        this(Integer.MAX_VALUE);
    }

    public TLinkedBlockingQueue(int capacity) {
        this.capacity = capacity;
    }

    @Override
    public synchronized boolean offer(E e) {
        if (items.size() >= capacity) {
            return false;
        }
        items.add(e);
        notifyAll();
        return true;
    }

    @Override
    public synchronized void put(E e) throws InterruptedException {
        while (items.size() >= capacity) {
            wait();
        }
        items.add(e);
        notifyAll();
    }

    @Override
    public synchronized boolean offer(E e, long timeout, TTimeUnit unit) throws InterruptedException {
        long deadline = System.currentTimeMillis() + unit.toMillis(timeout);
        while (items.size() >= capacity) {
            long left = deadline - System.currentTimeMillis();
            if (left <= 0) {
                return false;
            }
            wait(left);
        }
        items.add(e);
        notifyAll();
        return true;
    }

    @Override
    public synchronized E take() throws InterruptedException {
        while (items.isEmpty()) {
            wait();
        }
        E e = items.poll();
        notifyAll();
        return e;
    }

    @Override
    public synchronized E poll(long timeout, TTimeUnit unit) throws InterruptedException {
        long deadline = System.currentTimeMillis() + unit.toMillis(timeout);
        while (items.isEmpty()) {
            long left = deadline - System.currentTimeMillis();
            if (left <= 0) {
                return null;
            }
            wait(left);
        }
        E e = items.poll();
        notifyAll();
        return e;
    }

    @Override
    public synchronized E poll() {
        E e = items.poll();
        if (e != null) {
            notifyAll();
        }
        return e;
    }

    @Override
    public synchronized E peek() {
        return items.peek();
    }

    @Override
    public synchronized int size() {
        return items.size();
    }

    @Override
    public synchronized int remainingCapacity() {
        return capacity - items.size();
    }

    @Override
    public synchronized int drainTo(TCollection<? super E> c) {
        return drainTo(c, Integer.MAX_VALUE);
    }

    @Override
    public synchronized int drainTo(TCollection<? super E> c, int max) {
        int n = 0;
        while (n < max && !items.isEmpty()) {
            c.add(items.poll());
            n++;
        }
        if (n > 0) {
            notifyAll();
        }
        return n;
    }

    @Override
    public synchronized TIterator<E> iterator() {
        java.util.Iterator<E> it = new ArrayList<>(items).iterator();
        return new TIterator<E>() {
            @Override
            public boolean hasNext() {
                return it.hasNext();
            }

            @Override
            public E next() {
                return it.next();
            }

            @Override
            public void remove() {
                throw new UnsupportedOperationException();
            }
        };
    }
}
