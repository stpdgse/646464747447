package org.teavm.classlib.java.util.concurrent;

/** Timers are sleeping fibers; adequate for the few periodic jobs the game schedules. */
public class TScheduledThreadPoolExecutor extends TThreadPoolExecutor implements TScheduledExecutorService {
    public TScheduledThreadPoolExecutor(int core) {
        super(Integer.MAX_VALUE, null);
        inline(false); // timer loops sleep, so each needs its own fiber
    }

    public TScheduledThreadPoolExecutor(int core, TThreadFactory factory) {
        super(Integer.MAX_VALUE, factory);
        inline(false);
    }

    private final class Task extends TFutureTask<Object> implements TScheduledFuture<Object> {
        Task(Runnable r) {
            super(r, null);
        }
    }

    private TScheduledFuture<?> start(Runnable body, long initial, long period, boolean fixedRate,
            TTimeUnit unit, boolean repeat) {
        Task handle = new Task(() -> { });
        TRunnableLoop loop = new TRunnableLoop(body, initial, period, fixedRate, unit, repeat, handle);
        execute(loop::run);
        return handle;
    }

    private final class TRunnableLoop {
        final Runnable body;
        final long initial;
        final long period;
        final boolean fixedRate;
        final TTimeUnit unit;
        final boolean repeat;
        final Task handle;

        TRunnableLoop(Runnable body, long initial, long period, boolean fixedRate, TTimeUnit unit,
                boolean repeat, Task handle) {
            this.body = body;
            this.initial = initial;
            this.period = period;
            this.fixedRate = fixedRate;
            this.unit = unit;
            this.repeat = repeat;
            this.handle = handle;
        }

        void run() {
            try {
                Thread.sleep(unit.toMillis(initial));
                long next = System.currentTimeMillis();
                while (!handle.isDone() && !isShutdown()) {
                    long start = System.currentTimeMillis();
                    body.run();
                    if (!repeat) {
                        handle.run();
                        return;
                    }
                    long wait = unit.toMillis(period);
                    if (fixedRate) {
                        next += wait;
                        wait = Math.max(0, next - System.currentTimeMillis());
                    }
                    Thread.sleep(wait);
                }
            } catch (InterruptedException e) {
                // cancelled
            }
        }
    }

    @Override
    public TScheduledFuture<?> schedule(Runnable command, long delay, TTimeUnit unit) {
        return start(command, delay, 0, false, unit, false);
    }

    @Override
    public TScheduledFuture<?> scheduleAtFixedRate(Runnable command, long initialDelay, long period,
            TTimeUnit unit) {
        return start(command, initialDelay, period, true, unit, true);
    }

    @Override
    public TScheduledFuture<?> scheduleWithFixedDelay(Runnable command, long initialDelay, long delay,
            TTimeUnit unit) {
        return start(command, initialDelay, delay, false, unit, true);
    }
}
