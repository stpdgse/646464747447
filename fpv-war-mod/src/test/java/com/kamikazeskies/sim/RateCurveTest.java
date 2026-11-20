package com.kamikazeskies.sim;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class RateCurveTest {
    private final RateCurve curve = new RateCurve(100, 700, 0.5);

    @Test
    void zeroStickGivesZeroRate() {
        assertEquals(0.0, curve.rate(0.0), 1e-12);
    }

    @Test
    void fullStickGivesMaxRate() {
        assertEquals(700.0, curve.rate(1.0), 1e-9);
        assertEquals(-700.0, curve.rate(-1.0), 1e-9);
    }

    @Test
    void isOddSymmetricAndMonotonic() {
        double prev = -Double.MAX_VALUE;
        for (int i = -100; i <= 100; i++) {
            double x = i / 100.0;
            double r = curve.rate(x);
            assertEquals(-curve.rate(-x), r, 1e-9, "odd symmetry at " + x);
            assertTrue(r >= prev, "monotonic at " + x);
            prev = r;
        }
    }

    @Test
    void inputBeyondRangeIsClamped() {
        assertEquals(curve.rate(1.0), curve.rate(5.0), 1e-12);
        assertEquals(curve.rate(-1.0), curve.rate(-5.0), 1e-12);
    }

    @Test
    void expoSoftensTheMiddleOfTheStick() {
        RateCurve linear = new RateCurve(100, 700, 0.0);
        RateCurve expo = new RateCurve(100, 700, 1.0);
        assertTrue(expo.rate(0.5) < linear.rate(0.5));
    }

    @Test
    void rejectsInvalidParameters() {
        assertThrows(IllegalArgumentException.class, () -> new RateCurve(-1, 600, 0.3));
        assertThrows(IllegalArgumentException.class, () -> new RateCurve(700, 600, 0.3));
        assertThrows(IllegalArgumentException.class, () -> new RateCurve(100, 600, 1.5));
    }
}
