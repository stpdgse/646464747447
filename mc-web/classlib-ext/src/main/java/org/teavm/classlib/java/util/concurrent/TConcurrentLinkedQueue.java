package org.teavm.classlib.java.util.concurrent;

import java.util.AbstractQueue;
import java.util.ArrayDeque;
import java.util.Iterator;
import java.util.Queue;

public class TConcurrentLinkedQueue<E> extends AbstractQueue<E> implements Queue<E> {
    private final ArrayDeque<E> items = new ArrayDeque<>();

    public TConcurrentLinkedQueue() {
    }

    public TConcurrentLinkedQueue(java.util.Collection<? extends E> c) {
        for (E e : c) {
            offer(e);
        }
    }

    @Override
    public synchronized boolean offer(E e) {
        items.add(e);
        return true;
    }

    @Override
    public synchronized E poll() {
        return items.poll();
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
    public synchronized Iterator<E> iterator() {
        return new java.util.ArrayList<>(items).iterator();
    }
}
