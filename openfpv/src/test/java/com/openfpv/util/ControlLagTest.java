package com.openfpv.util;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertNotNull;
import static org.junit.Assert.assertNull;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class ControlLagTest {
    private static ControlLag.Sticks s(float pitch) {
        return new ControlLag.Sticks(pitch, 0, 0, 0);
    }

    @Test
    public void lagScalesWithSignalQuality() {
        assertEquals(0, ControlLag.lagTicks(1f, 10));
        assertEquals(5, ControlLag.lagTicks(0.5f, 10));
        assertEquals(10, ControlLag.lagTicks(0f, 10));
        assertEquals(10, ControlLag.lagTicks(-3f, 10));   // clamped
        assertEquals(0, ControlLag.lagTicks(2f, 10));
        assertEquals(0, ControlLag.lagTicks(0f, 0));
    }

    @Test
    public void lossProbabilityScales() {
        assertEquals(0.0, ControlLag.lossProbability(1f, 0.3), 0);
        assertEquals(0.15, ControlLag.lossProbability(0.5f, 0.3), 1e-9);
        assertEquals(0.3, ControlLag.lossProbability(0f, 0.3), 1e-9);
    }

    @Test
    public void perfectSignalDeliversImmediately() {
        ControlLag lag = new ControlLag(1);
        assertTrue(lag.push(100, s(0.5f), 1f, 10, 0.3));
        ControlLag.Sticks out = lag.poll(100);
        assertNotNull(out);
        assertEquals(0.5f, out.pitch, 0f);
        assertNull(lag.poll(100));
    }

    @Test
    public void weakSignalDelaysByTheRightNumberOfTicks() {
        ControlLag lag = new ControlLag(1);
        lag.push(100, s(1f), 0.5f, 10, 0.0);   // lag 5, no loss
        for (long t = 100; t < 105; t++) assertNull("too early at " + t, lag.poll(t));
        assertNotNull(lag.poll(105));
    }

    @Test
    public void packetsStayInOrderWhenTheLinkRecovers() {
        ControlLag lag = new ControlLag(1);
        lag.push(0, s(0.1f), 0f, 10, 0.0);     // due at 10
        lag.push(1, s(0.2f), 1f, 10, 0.0);     // would be due at 1, must not overtake
        assertNull(lag.poll(5));
        ControlLag.Sticks out = lag.poll(10);
        assertEquals(0.2f, out.pitch, 0f);      // both arrived; newest wins
    }

    @Test
    public void lossRateMatchesProbabilityOverManyPackets() {
        ControlLag lag = new ControlLag(42);
        int lost = 0, n = 20000;
        for (int i = 0; i < n; i++) {
            if (!lag.push(i, s(0f), 0f, 0, 0.3)) lost++;
            lag.poll(i);
        }
        assertEquals(0.3, lost / (double) n, 0.02);
    }

    @Test
    public void noLossAtPerfectSignal() {
        ControlLag lag = new ControlLag(7);
        for (int i = 0; i < 5000; i++) assertTrue(lag.push(i, s(0f), 1f, 10, 0.9));
    }

    @Test
    public void clearDropsEverything() {
        ControlLag lag = new ControlLag(1);
        lag.push(0, s(1f), 0f, 10, 0.0);
        lag.clear();
        assertNull(lag.poll(1000));
    }
}
