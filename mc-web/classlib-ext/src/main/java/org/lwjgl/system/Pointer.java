package org.lwjgl.system;

public interface Pointer {
    int POINTER_SIZE = 8;
    int POINTER_SHIFT = 3;
    int CLONG_SIZE = 8;
    int CLONG_SHIFT = 3;
    boolean BITS32 = false;
    boolean BITS64 = true;

    long address();

    abstract class Default implements Pointer {
        protected final long address;

        protected Default(long address) {
            this.address = address;
        }

        @Override
        public long address() {
            return address;
        }

        @Override
        public boolean equals(Object o) {
            // Written in two steps: TeaVM 0.16 miscompiles a long compare on an interface call inside &&.
            if (!(o instanceof Pointer)) {
                return false;
            }
            long other = ((Pointer) o).address();
            return Long.compare(other, address) == 0;
        }

        @Override
        public int hashCode() {
            return (int) (address ^ (address >>> 32));
        }
    }
}
