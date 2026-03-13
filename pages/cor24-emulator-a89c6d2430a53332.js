/**
 * WASM-accessible CPU wrapper backed by EmulatorCore
 */
export class WasmCpu {
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        WasmCpuFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wasmcpu_free(ptr, 0);
    }
    /**
     * Assemble source code and load into memory
     * @param {string} source
     * @returns {any}
     */
    assemble(source) {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.wasmcpu_assemble(this.__wbg_ptr, ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return takeFromExternrefTable0(ret[0]);
    }
    /**
     * Clear UART output buffer
     */
    clear_uart_output() {
        wasm.wasmcpu_clear_uart_output(this.__wbg_ptr);
    }
    /**
     * Get the assembled lines for display
     * @returns {string[]}
     */
    get_assembled_lines() {
        const ret = wasm.wasmcpu_get_assembled_lines(this.__wbg_ptr);
        var v1 = getArrayJsValueFromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get condition flag
     * @returns {boolean}
     */
    get_c_flag() {
        const ret = wasm.wasmcpu_get_c_flag(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get condition flag (alias for get_c_flag())
     * @returns {boolean}
     */
    get_condition_flag() {
        const ret = wasm.wasmcpu_get_condition_flag(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Get the current instruction disassembly (uses EmulatorCore's disassembler)
     * @returns {string}
     */
    get_current_instruction() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmcpu_get_current_instruction(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get instruction count as u32 (truncated from u64)
     * @returns {number}
     */
    get_instruction_count() {
        const ret = wasm.wasmcpu_get_instruction_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get LED value (alias for get_leds())
     * @returns {number}
     */
    get_led_value() {
        const ret = wasm.wasmcpu_get_led_value(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get LED state (8 bits)
     * @returns {number}
     */
    get_leds() {
        const ret = wasm.wasmcpu_get_leds(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get memory slice as bytes
     * @param {number} start
     * @param {number} len
     * @returns {Uint8Array}
     */
    get_memory_slice(start, len) {
        const ret = wasm.wasmcpu_get_memory_slice(this.__wbg_ptr, start, len);
        var v1 = getArrayU8FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 1, 1);
        return v1;
    }
    /**
     * Get program counter (alias for pc())
     * @returns {number}
     */
    get_pc() {
        const ret = wasm.wasmcpu_get_pc(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get the end address of loaded program (highest address written)
     * @returns {number}
     */
    get_program_end() {
        const ret = wasm.wasmcpu_get_program_end(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get all register values as an array
     * @returns {Uint32Array}
     */
    get_registers() {
        const ret = wasm.wasmcpu_get_registers(this.__wbg_ptr);
        var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get switch state (8 bits)
     * @returns {number}
     */
    get_switches() {
        const ret = wasm.wasmcpu_get_switches(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get UART output buffer
     * @returns {string}
     */
    get_uart_output() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.wasmcpu_get_uart_output(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Hard reset - clears memory too
     */
    hard_reset() {
        wasm.wasmcpu_hard_reset(this.__wbg_ptr);
    }
    /**
     * Get instruction count
     * @returns {bigint}
     */
    instruction_count() {
        const ret = wasm.wasmcpu_instruction_count(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Check if CPU is halted
     * @returns {boolean}
     */
    is_halted() {
        const ret = wasm.wasmcpu_is_halted(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Create a new CPU
     */
    constructor() {
        const ret = wasm.wasmcpu_new();
        this.__wbg_ptr = ret >>> 0;
        WasmCpuFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Get program counter
     * @returns {number}
     */
    pc() {
        const ret = wasm.wasmcpu_pc(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Read a byte from memory (alias for read_memory())
     * @param {number} addr
     * @returns {number}
     */
    read_byte(addr) {
        const ret = wasm.wasmcpu_read_byte(this.__wbg_ptr, addr);
        return ret;
    }
    /**
     * Read memory byte
     * @param {number} addr
     * @returns {number}
     */
    read_memory(addr) {
        const ret = wasm.wasmcpu_read_memory(this.__wbg_ptr, addr);
        return ret;
    }
    /**
     * Read a register value
     * @param {number} reg
     * @returns {number}
     */
    read_register(reg) {
        const ret = wasm.wasmcpu_read_register(this.__wbg_ptr, reg);
        return ret >>> 0;
    }
    /**
     * Reset the CPU to initial state (preserves memory)
     */
    reset() {
        wasm.wasmcpu_reset(this.__wbg_ptr);
    }
    /**
     * Run until halt or error
     */
    run() {
        const ret = wasm.wasmcpu_run(this.__wbg_ptr);
        if (ret[1]) {
            throw takeFromExternrefTable0(ret[0]);
        }
    }
    /**
     * Run a batch of instructions (for animated execution)
     * @param {number} max_instructions
     * @returns {boolean}
     */
    run_batch(max_instructions) {
        const ret = wasm.wasmcpu_run_batch(this.__wbg_ptr, max_instructions);
        return ret !== 0;
    }
    /**
     * Set switch state (simulates external switch input)
     * @param {number} value
     */
    set_switches(value) {
        wasm.wasmcpu_set_switches(this.__wbg_ptr, value);
    }
    /**
     * Check if should stop for LED output (for animation purposes)
     * @returns {boolean}
     */
    should_stop_for_led() {
        const ret = wasm.wasmcpu_should_stop_for_led(this.__wbg_ptr);
        return ret !== 0;
    }
    /**
     * Execute one instruction
     * @returns {boolean}
     */
    step() {
        const ret = wasm.wasmcpu_step(this.__wbg_ptr);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return ret[0] !== 0;
    }
    /**
     * Toggle a specific switch bit
     * @param {number} bit
     */
    toggle_switch(bit) {
        wasm.wasmcpu_toggle_switch(this.__wbg_ptr, bit);
    }
    /**
     * Send a character to UART RX (simulates input)
     * @param {string} c
     */
    uart_send_char(c) {
        const char0 = c.codePointAt(0);
        _assertChar(char0);
        wasm.wasmcpu_uart_send_char(this.__wbg_ptr, char0);
    }
}
if (Symbol.dispose) WasmCpu.prototype[Symbol.dispose] = WasmCpu.prototype.free;

/**
 * Get number of available challenges
 * @returns {number}
 */
export function get_challenge_count() {
    const ret = wasm.get_challenge_count();
    return ret >>> 0;
}

/**
 * Initialize the WASM module and mount Yew app
 */
export function init() {
    wasm.init();
}

/**
 * Validate a solution for a challenge
 * @param {number} challenge_id
 * @param {string} source
 * @returns {boolean}
 */
export function validate_challenge(challenge_id, source) {
    const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.validate_challenge(challenge_id, ptr0, len0);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return ret[0] !== 0;
}

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg_Error_4577686b3a6d9b3a: function(arg0, arg1) {
            const ret = Error(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_String_8564e559799eccda: function(arg0, arg1) {
            const ret = String(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_debug_string_ddde1867f49c2442: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_d633e708baf0d146: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_string_7debe47dc1e045c2: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_c18285b9fc34cb7d: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_number_get_5854912275df1894: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_throw_39bc967c0e5a9b58: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_b6d832240a919168: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_addEventListener_ba87252e1a7558be: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3, arg4);
        }, arguments); },
        __wbg_addEventListener_c4f780106c414839: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.addEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_body_4eb4906314b12ac0: function(arg0) {
            const ret = arg0.body;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_bottom_a78bc68536ba8b51: function(arg0) {
            const ret = arg0.bottom;
            return ret;
        },
        __wbg_bubbles_f49ceb31ef86f42b: function(arg0) {
            const ret = arg0.bubbles;
            return ret;
        },
        __wbg_cache_key_21c521bc3dd66e9b: function(arg0) {
            const ret = arg0.__yew_subtree_cache_key;
            return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
        },
        __wbg_cancelBubble_d83a60e9f0fe75c0: function(arg0) {
            const ret = arg0.cancelBubble;
            return ret;
        },
        __wbg_childNodes_10fe1c00547818b6: function(arg0) {
            const ret = arg0.childNodes;
            return ret;
        },
        __wbg_className_94f862c4f2ebfba8: function(arg0, arg1) {
            const ret = arg1.className;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_clearTimeout_113b1cde814ec762: function(arg0) {
            const ret = clearTimeout(arg0);
            return ret;
        },
        __wbg_clientHeight_cebf878e4f649e0d: function(arg0) {
            const ret = arg0.clientHeight;
            return ret;
        },
        __wbg_cloneNode_80f14b877dbe210f: function() { return handleError(function (arg0) {
            const ret = arg0.cloneNode();
            return ret;
        }, arguments); },
        __wbg_closest_f55e6d0fbe2b9d8c: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.closest(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_composedPath_80c7136c67c78587: function(arg0) {
            const ret = arg0.composedPath();
            return ret;
        },
        __wbg_createElementNS_ec037330952e0beb: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            const ret = arg0.createElementNS(arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
            return ret;
        }, arguments); },
        __wbg_createElement_c28be812ac2ffe84: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.createElement(getStringFromWasm0(arg1, arg2));
            return ret;
        }, arguments); },
        __wbg_createTextNode_608e1a15d944e7d8: function(arg0, arg1, arg2) {
            const ret = arg0.createTextNode(getStringFromWasm0(arg1, arg2));
            return ret;
        },
        __wbg_document_0b7613236d782ccc: function(arg0) {
            const ret = arg0.document;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_error_4c20fd6d19d38f03: function(arg0, arg1) {
            var v0 = getArrayJsValueFromWasm0(arg0, arg1).slice();
            wasm.__wbindgen_free(arg0, arg1 * 4, 4);
            console.error(...v0);
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_from_d7e888a2e9063b32: function(arg0) {
            const ret = Array.from(arg0);
            return ret;
        },
        __wbg_getAttribute_0ea134ba0b840481: function(arg0, arg1, arg2, arg3) {
            const ret = arg1.getAttribute(getStringFromWasm0(arg2, arg3));
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_getBoundingClientRect_881837f23b7e503b: function(arg0) {
            const ret = arg0.getBoundingClientRect();
            return ret;
        },
        __wbg_getElementById_dff2c0f6070bc31a: function(arg0, arg1, arg2) {
            const ret = arg0.getElementById(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_get_f09c3a16f8848381: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_host_7e46e40ab022dab7: function(arg0) {
            const ret = arg0.host;
            return ret;
        },
        __wbg_innerWidth_97c0ba1978a6cef6: function() { return handleError(function (arg0) {
            const ret = arg0.innerWidth;
            return ret;
        }, arguments); },
        __wbg_insertBefore_8a8f9c2f8b2135a2: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.insertBefore(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_instanceof_Element_3db3f20fca28f450: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Element;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlElement_4e9f5820ff28f6f0: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_HtmlSelectElement_4cdbbd445bf24c56: function(arg0) {
            let result;
            try {
                result = arg0 instanceof HTMLSelectElement;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_ShadowRoot_845d1aa5e4cd52ce: function(arg0) {
            let result;
            try {
                result = arg0 instanceof ShadowRoot;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_instanceof_Window_4aba49e4d1a12365: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_is_1ad0660d6042ae31: function(arg0, arg1) {
            const ret = Object.is(arg0, arg1);
            return ret;
        },
        __wbg_key_659f8d2f3a028e75: function(arg0, arg1) {
            const ret = arg1.key;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_lastChild_0b22ed4916ab8cfb: function(arg0) {
            const ret = arg0.lastChild;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_left_271b4450c739c266: function(arg0) {
            const ret = arg0.left;
            return ret;
        },
        __wbg_length_a31e05262e09b7f8: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_listener_id_51707c1ea7d7f75c: function(arg0) {
            const ret = arg0.__yew_listener_id;
            return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
        },
        __wbg_log_3c5e4b64af29e724: function(arg0) {
            console.log(arg0);
        },
        __wbg_namespaceURI_b7a32ea8484e7500: function(arg0, arg1) {
            const ret = arg1.namespaceURI;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_new_92df58a8ec3bfb6b: function() {
            const ret = new Map();
            return ret;
        },
        __wbg_new_cbee8c0d5c479eac: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_ed69e637b553a997: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_nextSibling_2f5a8c04873ca6ea: function(arg0) {
            const ret = arg0.nextSibling;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_offsetTop_719c9a279c5b1c98: function(arg0) {
            const ret = arg0.offsetTop;
            return ret;
        },
        __wbg_offsetWidth_57ed72d7c6b1ad33: function(arg0) {
            const ret = arg0.offsetWidth;
            return ret;
        },
        __wbg_outerHTML_4c80438cfe61f290: function(arg0, arg1) {
            const ret = arg1.outerHTML;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_parentElement_45ca02e0f5967c4c: function(arg0) {
            const ret = arg0.parentElement;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_parentNode_ec260baa7551ace2: function(arg0) {
            const ret = arg0.parentNode;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_preventDefault_d8dbb4013b32560a: function(arg0) {
            arg0.preventDefault();
        },
        __wbg_querySelector_1a3f1db99442c13e: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.querySelector(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_queueMicrotask_2c8dfd1056f24fdc: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_8985ad63815852e7: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_removeAttribute_ec7cfe3f3feaad6c: function() { return handleError(function (arg0, arg1, arg2) {
            arg0.removeAttribute(getStringFromWasm0(arg1, arg2));
        }, arguments); },
        __wbg_removeChild_09777b63122c7c9f: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.removeChild(arg1);
            return ret;
        }, arguments); },
        __wbg_removeEventListener_357b0bf9803ecae1: function() { return handleError(function (arg0, arg1, arg2, arg3) {
            arg0.removeEventListener(getStringFromWasm0(arg1, arg2), arg3);
        }, arguments); },
        __wbg_resolve_5d61e0d10c14730a: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_scrollTop_710343763bfbadb5: function(arg0) {
            const ret = arg0.scrollTop;
            return ret;
        },
        __wbg_setAttribute_52376c4b548b7c58: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
        }, arguments); },
        __wbg_setTimeout_ef24d2fc3ad97385: function() { return handleError(function (arg0, arg1) {
            const ret = setTimeout(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_set_4c81cfb5dc3a333c: function(arg0, arg1, arg2) {
            arg0[arg1 >>> 0] = arg2;
        },
        __wbg_set_6be42768c690e380: function(arg0, arg1, arg2) {
            arg0[arg1] = arg2;
        },
        __wbg_set_bad5c505cc70b5f8: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_cache_key_274f2145fcb4390c: function(arg0, arg1) {
            arg0.__yew_subtree_cache_key = arg1 >>> 0;
        },
        __wbg_set_capture_a853bd3db5681e0b: function(arg0, arg1) {
            arg0.capture = arg1 !== 0;
        },
        __wbg_set_cfc6de03f990decf: function(arg0, arg1, arg2) {
            const ret = arg0.set(arg1, arg2);
            return ret;
        },
        __wbg_set_checked_7cb17e2886338c2b: function(arg0, arg1) {
            arg0.checked = arg1 !== 0;
        },
        __wbg_set_innerHTML_a0467b14e22a2cb9: function(arg0, arg1, arg2) {
            arg0.innerHTML = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_listener_id_3046a6d6a2394ad9: function(arg0, arg1) {
            arg0.__yew_listener_id = arg1 >>> 0;
        },
        __wbg_set_nodeValue_c50dc3b50c2a8dab: function(arg0, arg1, arg2) {
            arg0.nodeValue = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_passive_5008572704d635e3: function(arg0, arg1) {
            arg0.passive = arg1 !== 0;
        },
        __wbg_set_scrollTop_b3df5fad1e9e525a: function(arg0, arg1) {
            arg0.scrollTop = arg1;
        },
        __wbg_set_subtree_id_fc80ace73ff247a7: function(arg0, arg1) {
            arg0.__yew_subtree_id = arg1 >>> 0;
        },
        __wbg_set_value_078f56ab8bf4ee14: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_value_286417191c84044a: function(arg0, arg1, arg2) {
            arg0.value = getStringFromWasm0(arg1, arg2);
        },
        __wbg_slice_34907933584427ec: function(arg0, arg1) {
            const ret = arg1.slice();
            const ptr1 = passArrayJsValueToWasm0(ret, wasm.__wbindgen_malloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_static_accessor_GLOBAL_THIS_14325d8cca34bb77: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_f3a1e69f9c5a7e8e: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_50cdb5b517789aca: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_WINDOW_d6c4126e4c244380: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_stopPropagation_189d059eaef22300: function(arg0) {
            arg0.stopPropagation();
        },
        __wbg_subtree_id_6a3200546ad613b1: function(arg0) {
            const ret = arg0.__yew_subtree_id;
            return isLikeNone(ret) ? 0x100000001 : (ret) >>> 0;
        },
        __wbg_target_adcfd1353c3de078: function(arg0) {
            const ret = arg0.target;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_textContent_763e4318c4623687: function(arg0, arg1) {
            const ret = arg1.textContent;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_then_f1c954fe00733701: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_value_161196e83c12d910: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_4865dc52408b4bc7: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_value_c00e81c270632bd1: function(arg0, arg1) {
            const ret = arg1.value;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_width_e2e02ec713b253dc: function(arg0) {
            const ret = arg0.width;
            return ret;
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 539, function: Function { arguments: [NamedExternref("KeyboardEvent")], shim_idx: 587, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h7624b15d7ed565ae, wasm_bindgen__convert__closures_____invoke__h4dc0df3a82744b86);
            return ret;
        },
        __wbindgen_cast_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 539, function: Function { arguments: [NamedExternref("MouseEvent")], shim_idx: 587, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h7624b15d7ed565ae, wasm_bindgen__convert__closures_____invoke__h4dc0df3a82744b86_1);
            return ret;
        },
        __wbindgen_cast_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 653, function: Function { arguments: [Ref(NamedExternref("Event"))], shim_idx: 647, ret: Unit, inner_ret: Some(Unit) }, mutable: false }) -> Externref`.
            const ret = makeClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__ha6f0ebf1ebfa14ca, wasm_bindgen__convert__closures________invoke__h5b333aab1bd1885b);
            return ret;
        },
        __wbindgen_cast_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 720, function: Function { arguments: [], shim_idx: 716, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__h079a115b00197028, wasm_bindgen__convert__closures_____invoke__h1788c5197982ae51);
            return ret;
        },
        __wbindgen_cast_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { dtor_idx: 721, function: Function { arguments: [Externref], shim_idx: 739, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm.wasm_bindgen__closure__destroy__hf97f17c9c95398e5, wasm_bindgen__convert__closures_____invoke__h0bcb239972d1a1d5);
            return ret;
        },
        __wbindgen_cast_0000000000000006: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_cast_0000000000000007: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./cor24-emulator_bg.js": import0,
    };
}

function wasm_bindgen__convert__closures_____invoke__h1788c5197982ae51(arg0, arg1) {
    wasm.wasm_bindgen__convert__closures_____invoke__h1788c5197982ae51(arg0, arg1);
}

function wasm_bindgen__convert__closures_____invoke__h4dc0df3a82744b86(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h4dc0df3a82744b86(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h4dc0df3a82744b86_1(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures_____invoke__h4dc0df3a82744b86_1(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures________invoke__h5b333aab1bd1885b(arg0, arg1, arg2) {
    wasm.wasm_bindgen__convert__closures________invoke__h5b333aab1bd1885b(arg0, arg1, arg2);
}

function wasm_bindgen__convert__closures_____invoke__h0bcb239972d1a1d5(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen__convert__closures_____invoke__h0bcb239972d1a1d5(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

const WasmCpuFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wasmcpu_free(ptr >>> 0, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

function _assertChar(c) {
    if (typeof(c) === 'number' && (c >= 0x110000 || (c >= 0xD800 && c < 0xE000))) throw new Error(`expected a valid Unicode scalar value, found ${c}`);
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => state.dtor(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayJsValueFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    wasm.__externref_drop_slice(ptr, len);
    return result;
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.byteLength === 0) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        try {
            return f(state.a, state.b, ...args);
        } finally {
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            state.dtor(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function makeMutClosure(arg0, arg1, dtor, f) {
    const state = { a: arg0, b: arg1, cnt: 1, dtor };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            state.dtor(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArrayJsValueToWasm0(array, malloc) {
    const ptr = malloc(array.length * 4, 4) >>> 0;
    for (let i = 0; i < array.length; i++) {
        const add = addToExternrefTable0(array[i]);
        getDataViewMemory0().setUint32(ptr + 4 * i, add, true);
    }
    WASM_VECTOR_LEN = array.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('cor24-emulator_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
