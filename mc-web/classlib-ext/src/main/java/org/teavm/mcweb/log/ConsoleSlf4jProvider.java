package org.teavm.mcweb.log;

import org.slf4j.ILoggerFactory;
import org.slf4j.IMarkerFactory;
import org.slf4j.Logger;
import org.slf4j.Marker;
import org.slf4j.event.Level;
import org.slf4j.helpers.AbstractLogger;
import org.slf4j.helpers.BasicMarkerFactory;
import org.slf4j.helpers.MessageFormatter;
import org.slf4j.helpers.NOPMDCAdapter;
import org.slf4j.spi.MDCAdapter;
import org.slf4j.spi.SLF4JServiceProvider;

/** SLF4J backend that writes to System.out/err, which the page forwards to the browser console. */
public class ConsoleSlf4jProvider implements SLF4JServiceProvider {
    private final ILoggerFactory factory = name -> new ConsoleLogger(name);
    private final IMarkerFactory markers = new BasicMarkerFactory();
    private final MDCAdapter mdc = new NOPMDCAdapter();

    @Override
    public ILoggerFactory getLoggerFactory() {
        return factory;
    }

    @Override
    public IMarkerFactory getMarkerFactory() {
        return markers;
    }

    @Override
    public MDCAdapter getMDCAdapter() {
        return mdc;
    }

    @Override
    public String getRequestedApiVersion() {
        return "2.0.99";
    }

    @Override
    public void initialize() {
    }

    static final class ConsoleLogger extends AbstractLogger {
        ConsoleLogger(String name) {
            this.name = name;
        }

        @Override
        protected String getFullyQualifiedCallerName() {
            return null;
        }

        @Override
        protected void handleNormalizedLoggingCall(Level level, Marker marker, String msg, Object[] args,
                Throwable t) {
            String text = args == null || args.length == 0 ? msg : MessageFormatter.arrayFormat(msg, args).getMessage();
            String line = "[" + level + "] " + (name == null ? "" : name.substring(name.lastIndexOf('.') + 1)) + ": "
                    + text;
            // Straight to the browser console: System.out may be redirected back into this logger.
            if (level == Level.ERROR) {
                org.teavm.mcweb.host.WebHost.consoleError(line);
            } else if (level == Level.WARN) {
                org.teavm.mcweb.host.WebHost.consoleWarn(line);
            } else {
                org.teavm.mcweb.host.WebHost.consoleLog(line);
            }
            for (Throwable c = t; c != null; c = c.getCause() == c ? null : c.getCause()) {
                org.teavm.mcweb.host.WebHost.consoleError("    " + c);
            }
        }

        @Override
        public boolean isTraceEnabled() {
            return false;
        }

        @Override
        public boolean isTraceEnabled(Marker m) {
            return false;
        }

        @Override
        public boolean isDebugEnabled() {
            // Game code behind isDebugEnabled() inspects real stack traces, which TeaVM cannot provide.
            return false;
        }

        @Override
        public boolean isDebugEnabled(Marker m) {
            return false;
        }

        @Override
        public boolean isInfoEnabled() {
            return true;
        }

        @Override
        public boolean isInfoEnabled(Marker m) {
            return true;
        }

        @Override
        public boolean isWarnEnabled() {
            return true;
        }

        @Override
        public boolean isWarnEnabled(Marker m) {
            return true;
        }

        @Override
        public boolean isErrorEnabled() {
            return true;
        }

        @Override
        public boolean isErrorEnabled(Marker m) {
            return true;
        }
    }
}
