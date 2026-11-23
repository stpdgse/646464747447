package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

import java.util.ArrayList;
import java.util.List;

/** A fixed sequence of struct objects with a position, like LWJGL's SdlStructBuffer. */
public abstract class SdlStructBuffer<T extends Struct<T>, SELF extends SdlStructBuffer<T, SELF>> {
    protected final List<T> items = new ArrayList<>();
    protected int position;

    protected abstract T newElement();

    protected abstract SELF self();

    protected SELF fill(int count) {
        for (int i = 0; i < count; i++) {
            items.add(newElement());
        }
        return self();
    }

    public T get(int index) {
        return items.get(index);
    }

    public T get() {
        return items.get(position++);
    }

    public int capacity() {
        return items.size();
    }

    public int remaining() {
        return items.size() - position;
    }

    public SELF position(int p) {
        position = p;
        return self();
    }

    public int position() {
        return position;
    }

    protected T current() {
        return items.get(position);
    }

    public void free() {
    }
}
