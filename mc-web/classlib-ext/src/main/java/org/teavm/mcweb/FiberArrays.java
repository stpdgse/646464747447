package org.teavm.mcweb;

/**
 * Array growth for TeaVM's fiber stack. In this program nearly every method is compiled as a
 * coroutine (asynchrony spreads through virtual calls), including {@code Arrays.copyOf}. A fiber
 * that grows its saved-stack arrays while suspending then calls the coroutine version of
 * {@code copyOf}, which tries to save its own state, which grows the arrays again, forever. These
 * helpers only use plain array creation and {@code System.arraycopy}, so they can never suspend.
 */
public final class FiberArrays {
    private FiberArrays() {
    }

    public static int[] copyOf(int[] source, int length) {
        int[] result = new int[length];
        System.arraycopy(source, 0, result, 0, Math.min(source.length, length));
        return result;
    }

    public static long[] copyOf(long[] source, int length) {
        long[] result = new long[length];
        System.arraycopy(source, 0, result, 0, Math.min(source.length, length));
        return result;
    }

    public static float[] copyOf(float[] source, int length) {
        float[] result = new float[length];
        System.arraycopy(source, 0, result, 0, Math.min(source.length, length));
        return result;
    }

    public static double[] copyOf(double[] source, int length) {
        double[] result = new double[length];
        System.arraycopy(source, 0, result, 0, Math.min(source.length, length));
        return result;
    }

    /** The fiber's object stack is exactly {@code Object[]}, so a plain {@code Object[]} is the right type. */
    public static Object[] copyOf(Object[] source, int length) {
        Object[] result = new Object[length];
        System.arraycopy(source, 0, result, 0, Math.min(source.length, length));
        return result;
    }
}
