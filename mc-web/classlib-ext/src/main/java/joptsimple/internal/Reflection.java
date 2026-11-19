package joptsimple.internal;

import java.io.File;
import java.lang.reflect.Constructor;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import joptsimple.ValueConverter;

/**
 * Replacement for jopt-simple's reflection helper. TeaVM cannot look up {@code valueOf} methods or
 * constructors reflectively, so converters for the handful of types the game's launcher uses are
 * written out explicitly.
 */
public final class Reflection {
    private Reflection() {
        throw new UnsupportedOperationException();
    }

    @SuppressWarnings("unchecked")
    public static <V> ValueConverter<V> findConverter(Class<V> type) {
        if (type == String.class) {
            return (ValueConverter<V>) converter(String.class, s -> s);
        }
        if (type == Integer.class || type == int.class) {
            return (ValueConverter<V>) converter(Integer.class, Integer::valueOf);
        }
        if (type == Long.class || type == long.class) {
            return (ValueConverter<V>) converter(Long.class, Long::valueOf);
        }
        if (type == Double.class || type == double.class) {
            return (ValueConverter<V>) converter(Double.class, Double::valueOf);
        }
        if (type == Float.class || type == float.class) {
            return (ValueConverter<V>) converter(Float.class, Float::valueOf);
        }
        if (type == Boolean.class || type == boolean.class) {
            return (ValueConverter<V>) converter(Boolean.class, Boolean::valueOf);
        }
        if (type == File.class) {
            return (ValueConverter<V>) converter(File.class, File::new);
        }
        if (type.isEnum()) {
            Object[] constants = type.getEnumConstants();
            return (ValueConverter<V>) converter((Class<Object>) type, s -> {
                for (Object c : constants) {
                    if (((Enum<?>) c).name().equalsIgnoreCase(s)) {
                        return c;
                    }
                }
                throw new IllegalArgumentException(s);
            });
        }
        throw new IllegalArgumentException(type + " is not a supported option value type in the browser port");
    }

    private interface Parse<T> {
        T apply(String s);
    }

    private static <T> ValueConverter<T> converter(Class<T> type, Parse<T> parse) {
        return new ValueConverter<T>() {
            @Override
            public T convert(String value) {
                return parse.apply(value);
            }

            @Override
            public Class<? extends T> valueType() {
                return type;
            }

            @Override
            public String valuePattern() {
                return null;
            }
        };
    }

    public static <T> T instantiate(Constructor<T> constructor, Object... args) {
        try {
            return constructor.newInstance(args);
        } catch (Exception ex) {
            throw reflectionException(ex);
        }
    }

    public static Object invoke(Method method, Object... args) {
        try {
            return method.invoke(null, args);
        } catch (Exception ex) {
            throw reflectionException(ex);
        }
    }

    @SuppressWarnings("unchecked")
    public static <V> V convertWith(ValueConverter<V> converter, String raw) {
        return converter == null ? (V) raw : converter.convert(raw);
    }

    private static RuntimeException reflectionException(Exception ex) {
        if (ex instanceof IllegalArgumentException) {
            return new ReflectionException(ex);
        }
        if (ex instanceof InvocationTargetException) {
            return new ReflectionException(ex.getCause());
        }
        if (ex instanceof RuntimeException) {
            return (RuntimeException) ex;
        }
        return new ReflectionException(ex);
    }
}
