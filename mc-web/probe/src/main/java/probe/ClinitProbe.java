package probe;

import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.ConcurrentMap;
import java.util.function.Function;

public class ClinitProbe {
    static final ConcurrentMap<String, Object> MAP = new ConcurrentHashMap<>();

    static Object make(String s) {
        return MAP.computeIfAbsent(s, (Function<String, Object>) k -> new Object());
    }

    static class Reg {
    static final Object K0 = make("k0");
        static final Object K1 = make("k1");
        static final Object K2 = make("k2");
        static final Object K3 = make("k3");
        static final Object K4 = make("k4");
        static final Object K5 = make("k5");
        static final Object K6 = make("k6");
        static final Object K7 = make("k7");
        static final Object K8 = make("k8");
        static final Object K9 = make("k9");
        static final Object K10 = make("k10");
        static final Object K11 = make("k11");
        static final Object K12 = make("k12");
        static final Object K13 = make("k13");
        static final Object K14 = make("k14");
        static final Object K15 = make("k15");
        static final Object K16 = make("k16");
        static final Object K17 = make("k17");
        static final Object K18 = make("k18");
        static final Object K19 = make("k19");
        static final Object K20 = make("k20");
        static final Object K21 = make("k21");
        static final Object K22 = make("k22");
        static final Object K23 = make("k23");
        static final Object K24 = make("k24");
        static final Object K25 = make("k25");
        static final Object K26 = make("k26");
        static final Object K27 = make("k27");
        static final Object K28 = make("k28");
        static final Object K29 = make("k29");
        static final Object K30 = make("k30");
        static final Object K31 = make("k31");
        static final Object K32 = make("k32");
        static final Object K33 = make("k33");
        static final Object K34 = make("k34");
        static final Object K35 = make("k35");
        static final Object K36 = make("k36");
        static final Object K37 = make("k37");
        static final Object K38 = make("k38");
        static final Object K39 = make("k39");
        static final Object K40 = make("k40");
        static final Object K41 = make("k41");
        static final Object K42 = make("k42");
        static final Object K43 = make("k43");
        static final Object K44 = make("k44");
        static final Object K45 = make("k45");
        static final Object K46 = make("k46");
        static final Object K47 = make("k47");
        static final Object K48 = make("k48");
        static final Object K49 = make("k49");
        static final Object K50 = make("k50");
        static final Object K51 = make("k51");
        static final Object K52 = make("k52");
        static final Object K53 = make("k53");
        static final Object K54 = make("k54");
        static final Object K55 = make("k55");
        static final Object K56 = make("k56");
        static final Object K57 = make("k57");
        static final Object K58 = make("k58");
        static final Object K59 = make("k59");
        static final Object K60 = make("k60");
        static final Object K61 = make("k61");
        static final Object K62 = make("k62");
        static final Object K63 = make("k63");
        static final Object K64 = make("k64");
        static final Object K65 = make("k65");
        static final Object K66 = make("k66");
        static final Object K67 = make("k67");
        static final Object K68 = make("k68");
        static final Object K69 = make("k69");
        static final Object K70 = make("k70");
        static final Object K71 = make("k71");
        static final Object K72 = make("k72");
        static final Object K73 = make("k73");
        static final Object K74 = make("k74");
        static final Object K75 = make("k75");
        static final Object K76 = make("k76");
        static final Object K77 = make("k77");
        static final Object K78 = make("k78");
        static final Object K79 = make("k79");
        static final Object K80 = make("k80");
        static final Object K81 = make("k81");
        static final Object K82 = make("k82");
        static final Object K83 = make("k83");
        static final Object K84 = make("k84");
        static final Object K85 = make("k85");
        static final Object K86 = make("k86");
        static final Object K87 = make("k87");
        static final Object K88 = make("k88");
        static final Object K89 = make("k89");
        static final Object K90 = make("k90");
        static final Object K91 = make("k91");
        static final Object K92 = make("k92");
        static final Object K93 = make("k93");
        static final Object K94 = make("k94");
        static final Object K95 = make("k95");
        static final Object K96 = make("k96");
        static final Object K97 = make("k97");
        static final Object K98 = make("k98");
        static final Object K99 = make("k99");
        static final Object K100 = make("k100");
        static final Object K101 = make("k101");
        static final Object K102 = make("k102");
        static final Object K103 = make("k103");
        static final Object K104 = make("k104");
        static final Object K105 = make("k105");
        static final Object K106 = make("k106");
        static final Object K107 = make("k107");
        static final Object K108 = make("k108");
        static final Object K109 = make("k109");
        static final Object K110 = make("k110");
        static final Object K111 = make("k111");
        static final Object K112 = make("k112");
        static final Object K113 = make("k113");
        static final Object K114 = make("k114");
        static final Object K115 = make("k115");
        static final Object K116 = make("k116");
        static final Object K117 = make("k117");
        static final Object K118 = make("k118");
        static final Object K119 = make("k119");
        static final Object K120 = make("k120");
        static final Object K121 = make("k121");
        static final Object K122 = make("k122");
        static final Object K123 = make("k123");
        static final Object K124 = make("k124");
        static final Object K125 = make("k125");
        static final Object K126 = make("k126");
        static final Object K127 = make("k127");
        static final Object K128 = make("k128");
        static final Object K129 = make("k129");
        static final Object K130 = make("k130");
        static final Object K131 = make("k131");
        static final Object K132 = make("k132");
        static final Object K133 = make("k133");
        static final Object K134 = make("k134");
        static final Object K135 = make("k135");
        static final Object K136 = make("k136");
        static final Object K137 = make("k137");
        static final Object K138 = make("k138");
        static final Object K139 = make("k139");
        static final Object K140 = make("k140");
        static final Object K141 = make("k141");
        static final Object K142 = make("k142");
        static final Object K143 = make("k143");
        static final Object K144 = make("k144");
        static final Object K145 = make("k145");
        static final Object K146 = make("k146");
        static final Object K147 = make("k147");
        static final Object K148 = make("k148");
        static final Object K149 = make("k149");
        static final Object K150 = make("k150");
        static final Object K151 = make("k151");
        static final Object K152 = make("k152");
        static final Object K153 = make("k153");
        static final Object K154 = make("k154");
        static final Object K155 = make("k155");
        static final Object K156 = make("k156");
        static final Object K157 = make("k157");
        static final Object K158 = make("k158");
        static final Object K159 = make("k159");
    }

    /** A function that really can block: makes Function.apply "async" for TeaVM. */
    static class Sleepy implements Function<String, Object> {
        @Override
        public Object apply(String s) {
            try {
                Thread.sleep(5);
            } catch (InterruptedException e) {
                throw new RuntimeException(e);
            }
            return s;
        }
    }

    public static void main(String[] args) throws Exception {
        System.out.println("K0=" + Reg.K0 + " K100=" + Reg.K100 + " K159=" + Reg.K159);
        System.out.println("sleepy=" + new Sleepy().apply("x"));
    }
}
