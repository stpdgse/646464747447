package org.teavm.classlib.java.text;

import org.teavm.classlib.java.lang.TObject;

public final class TStringCharacterIterator extends TObject implements TCharacterIterator {
    private String text;
    private int begin;
    private int end;
    private int pos;

    public TStringCharacterIterator(String text) {
        this(text, 0);
    }

    public TStringCharacterIterator(String text, int pos) {
        this(text, 0, text.length(), pos);
    }

    public TStringCharacterIterator(String text, int begin, int end, int pos) {
        this.text = text;
        this.begin = begin;
        this.end = end;
        this.pos = pos;
    }

    public void setText(String text) {
        this.text = text;
        this.begin = 0;
        this.end = text.length();
        this.pos = 0;
    }

    @Override
    public char first() {
        pos = begin;
        return current();
    }

    @Override
    public char last() {
        pos = end > begin ? end - 1 : end;
        return current();
    }

    @Override
    public char setIndex(int p) {
        pos = p;
        return current();
    }

    @Override
    public char current() {
        return pos >= begin && pos < end ? text.charAt(pos) : DONE;
    }

    @Override
    public char next() {
        if (pos < end - 1) {
            pos++;
            return text.charAt(pos);
        }
        pos = end;
        return DONE;
    }

    @Override
    public char previous() {
        if (pos > begin) {
            pos--;
            return text.charAt(pos);
        }
        return DONE;
    }

    @Override
    public int getBeginIndex() {
        return begin;
    }

    @Override
    public int getEndIndex() {
        return end;
    }

    @Override
    public int getIndex() {
        return pos;
    }

    @Override
    public Object clone() {
        return new TStringCharacterIterator(text, begin, end, pos);
    }
}
