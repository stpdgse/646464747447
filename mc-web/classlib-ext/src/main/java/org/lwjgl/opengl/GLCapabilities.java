package org.lwjgl.opengl;

/**
 * Capability flags. The browser exposes WebGL2 (GLES 3.0), so core 3.3 behaviour is assumed and every
 * desktop extension is reported as absent, which makes the game choose its portable code paths.
 */
public final class GLCapabilities {
    public final boolean GL_ARB_base_instance = false;
    public final boolean GL_ARB_buffer_storage = false;
    public final boolean GL_ARB_clip_control = false;
    public final boolean GL_ARB_debug_output = false;
    public final boolean GL_ARB_direct_state_access = false;
    public final boolean GL_ARB_draw_indirect = false;
    public final boolean GL_ARB_multi_draw_indirect = false;
    public final boolean GL_ARB_shader_draw_parameters = false;
    public final boolean GL_ARB_vertex_attrib_binding = false;
    public final boolean GL_EXT_debug_label = false;
    public final boolean GL_EXT_texture_filter_anisotropic = false;
    public final boolean GL_KHR_debug = false;
    public final boolean OpenGL11 = true;
    public final boolean OpenGL20 = true;
    public final boolean OpenGL30 = true;
    public final boolean OpenGL31 = true;
    public final boolean OpenGL32 = true;
    public final boolean OpenGL33 = true;
    public final boolean OpenGL40 = false;
    public final boolean OpenGL43 = false;
    public final boolean OpenGL45 = false;
}
