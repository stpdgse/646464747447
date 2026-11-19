package org.teavm.classlib.java.util.concurrent;

public interface TScheduledExecutorService extends TExecutorService {
    TScheduledFuture<?> schedule(Runnable command, long delay, TTimeUnit unit);

    TScheduledFuture<?> scheduleAtFixedRate(Runnable command, long initialDelay, long period, TTimeUnit unit);

    TScheduledFuture<?> scheduleWithFixedDelay(Runnable command, long initialDelay, long delay, TTimeUnit unit);
}
