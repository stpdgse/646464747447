package org.lwjgl.util.freetype;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import org.lwjgl.PointerBuffer;
import org.lwjgl.system.Library;
import org.lwjgl.system.SharedLibrary;

/**
 * FreeType is not available in the browser port. Initialisation reports an error, so only custom
 * TrueType font providers (not the default bitmap fonts) are affected.
 */
public class FreeType {
    public static final int FT_ENCODING_UNICODE = 0x756E6963;
    private static final int ERROR = 1;

    public static SharedLibrary getLibrary() {
        return Library.loadNative("freetype", "freetype");
    }

    public static int FT_Init_FreeType(PointerBuffer library) {
        return ERROR;
    }

    public static int FT_Done_Library(long library) {
        return 0;
    }

    public static int FT_Done_Face(FT_Face face) {
        return 0;
    }

    public static String FT_Error_String(int error) {
        return "FreeType is not available in the browser port";
    }

    public static int FT_New_Memory_Face(long library, ByteBuffer data, long faceIndex, PointerBuffer face) {
        return ERROR;
    }

    public static int FT_Select_Charmap(FT_Face face, int encoding) {
        return ERROR;
    }

    public static int FT_Set_Pixel_Sizes(FT_Face face, int width, int height) {
        return ERROR;
    }

    public static int FT_Load_Glyph(FT_Face face, int glyph, int flags) {
        return ERROR;
    }

    public static long FT_Get_First_Char(FT_Face face, IntBuffer index) {
        return 0;
    }

    public static long FT_Get_Next_Char(FT_Face face, long charCode, IntBuffer index) {
        return 0;
    }

    public static String FT_Get_Font_Format(FT_Face face) {
        return null;
    }

    public static void FT_Set_Transform(FT_Face face, FT_Matrix matrix, FT_Vector delta) {
    }
}
