package com.mojang.text2speech;

/** Text-to-speech is not wired up in the browser port yet: a silent narrator. */
public interface Narrator {
    Narrator EMPTY = new Narrator() {
        @Override
        public void say(String msg, boolean interrupt, float volume) {
        }

        @Override
        public void clear() {
        }

        @Override
        public void destroy() {
        }
    };

    void say(String msg, boolean interrupt, float volume);

    void clear();

    default boolean active() {
        return false;
    }

    void destroy();

    static Narrator getNarrator() {
        return EMPTY;
    }
}
