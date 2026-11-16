let wasm;

const heap = new Array(128).fill(undefined);

heap.push(undefined, null, true, false);

function getObject(idx) { return heap[idx]; }

function isLikeNone(x) {
    return x === undefined || x === null;
}

let heap_next = heap.length;

function addHeapObject(obj) {
    if (heap_next === heap.length) heap.push(heap.length + 1);
    const idx = heap_next;
    heap_next = heap[idx];

    heap[idx] = obj;
    return idx;
}

const cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : { decode: () => { throw Error('TextDecoder not available') } } );

if (typeof TextDecoder !== 'undefined') { cachedTextDecoder.decode(); };

let cachedUint8ArrayMemory0 = null;

function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        wasm.__wbindgen_export_0(addHeapObject(e));
    }
}

let WASM_VECTOR_LEN = 0;

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder('utf-8') : { encode: () => { throw Error('TextEncoder not available') } } );

const encodeString = (typeof cachedTextEncoder.encodeInto === 'function'
    ? function (arg, view) {
    return cachedTextEncoder.encodeInto(arg, view);
}
    : function (arg, view) {
    const buf = cachedTextEncoder.encode(arg);
    view.set(buf);
    return {
        read: arg.length,
        written: buf.length
    };
});

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
        const ret = encodeString(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

let cachedDataViewMemory0 = null;

function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

function dropObject(idx) {
    if (idx < 132) return;
    heap[idx] = heap_next;
    heap_next = idx;
}

function takeObject(idx) {
    const ret = getObject(idx);
    dropObject(idx);
    return ret;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => {
    wasm.__wbindgen_export_3.get(state.dtor)(state.a, state.b)
});

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
            if (--state.cnt === 0) {
                wasm.__wbindgen_export_3.get(state.dtor)(a, state.b);
                CLOSURE_DTORS.unregister(state);
            } else {
                state.a = a;
            }
        }
    };
    real.original = state;
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

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
/**
 * @param {string} content_url
 * @returns {Promise<void>}
 */
export function boot(content_url) {
    const ptr0 = passStringToWasm0(content_url, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.boot(ptr0, len0);
    return takeObject(ret);
}

/**
 * Start from content text that the host already has (the desktop app passes its cached copy).
 * @param {string} text
 */
export function boot_with(text) {
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        const ptr0 = passStringToWasm0(text, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len0 = WASM_VECTOR_LEN;
        wasm.boot_with(retptr, ptr0, len0);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        if (r1) {
            throw takeObject(r0);
        }
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
    }
}

/**
 * @returns {string}
 */
export function content_version() {
    let deferred1_0;
    let deferred1_1;
    try {
        const retptr = wasm.__wbindgen_add_to_stack_pointer(-16);
        wasm.content_version(retptr);
        var r0 = getDataViewMemory0().getInt32(retptr + 4 * 0, true);
        var r1 = getDataViewMemory0().getInt32(retptr + 4 * 1, true);
        deferred1_0 = r0;
        deferred1_1 = r1;
        return getStringFromWasm0(r0, r1);
    } finally {
        wasm.__wbindgen_add_to_stack_pointer(16);
        wasm.__wbindgen_export_4(deferred1_0, deferred1_1, 1);
    }
}

function __wbg_adapter_30(arg0, arg1, arg2, arg3) {
    wasm.__wbindgen_export_5(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

function __wbg_adapter_33(arg0, arg1, arg2) {
    wasm.__wbindgen_export_6(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_36(arg0, arg1) {
    wasm.__wbindgen_export_7(arg0, arg1);
}

function __wbg_adapter_39(arg0, arg1, arg2) {
    wasm.__wbindgen_export_8(arg0, arg1, addHeapObject(arg2));
}

function __wbg_adapter_397(arg0, arg1, arg2, arg3) {
    wasm.__wbindgen_export_9(arg0, arg1, addHeapObject(arg2), addHeapObject(arg3));
}

const __wbindgen_enum_ScrollBehavior = ["auto", "instant", "smooth"];

const __wbindgen_enum_ScrollLogicalPosition = ["start", "center", "end", "nearest"];

const __wbindgen_enum_ScrollRestoration = ["auto", "manual"];

const __wbindgen_enum_VisibilityState = ["hidden", "visible"];

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);

            } catch (e) {
                if (module.headers.get('Content-Type') != 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else {
                    throw e;
                }
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
}

function __wbg_get_imports() {
    const imports = {};
    imports.wbg = {};
    imports.wbg.__wbg_activeElement_367599fdfa7ad115 = function(arg0) {
        const ret = getObject(arg0).activeElement;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_addEventListener_7b8506ed3daef7d5 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), arg4 !== 0);
    }, arguments) };
    imports.wbg.__wbg_addEventListener_84ae3eac6e15480a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
    }, arguments) };
    imports.wbg.__wbg_addEventListener_90e553fdce254421 = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        getObject(arg0).addEventListener(getStringFromWasm0(arg1, arg2), getObject(arg3));
    }, arguments) };
    imports.wbg.__wbg_add_9b5191a4a4f767dc = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).add(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_addedNodes_02074aeb0a0af278 = function(arg0) {
        const ret = getObject(arg0).addedNodes;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_after_7474586172c50ce7 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).after(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_all_d5bf227e8f68795d = function(arg0) {
        const ret = Promise.all(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_altKey_c33c03aed82e4275 = function(arg0) {
        const ret = getObject(arg0).altKey;
        return ret;
    };
    imports.wbg.__wbg_appendChild_8204974b7328bf98 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).appendChild(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_apply_36be6a55257c99bf = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).apply(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_body_942ea927546a04ba = function(arg0) {
        const ret = getObject(arg0).body;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_bottom_79b03e9c3d6f4e1e = function(arg0) {
        const ret = getObject(arg0).bottom;
        return ret;
    };
    imports.wbg.__wbg_call_672a4d21634d4a24 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).call(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_call_7cccdd69e0791ae2 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).call(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_checked_0591091c28a685f0 = function(arg0) {
        const ret = getObject(arg0).checked;
        return ret;
    };
    imports.wbg.__wbg_children_9b8d0778c0b363f8 = function(arg0) {
        const ret = getObject(arg0).children;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_classList_3fa995ef71da9e8e = function(arg0) {
        const ret = getObject(arg0).classList;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_clearInterval_ad2594253cc39c4b = function(arg0, arg1) {
        getObject(arg0).clearInterval(arg1);
    };
    imports.wbg.__wbg_clearTimeout_b2651b7485c58446 = function(arg0, arg1) {
        getObject(arg0).clearTimeout(arg1);
    };
    imports.wbg.__wbg_clipboard_93f8aa8cc426db44 = function(arg0) {
        const ret = getObject(arg0).clipboard;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_close_9135df2442e72d26 = function(arg0) {
        getObject(arg0).close();
    };
    imports.wbg.__wbg_closest_73ed23f4aa9c0b46 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).closest(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_commonAncestorContainer_8b9bb61cb6ae69d3 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).commonAncestorContainer;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_contains_3361c7eda6c95afd = function(arg0, arg1) {
        const ret = getObject(arg0).contains(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_contains_687eea5148ddb87c = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).contains(getStringFromWasm0(arg1, arg2));
        return ret;
    };
    imports.wbg.__wbg_createElement_8c9931a732ee2fea = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).createElement(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createRange_955c5d31fcaabb2c = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).createRange();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_createTreeWalker_312ba17811b7a369 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).createTreeWalker(getObject(arg1), arg2 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_ctrlKey_1e826e468105ac11 = function(arg0) {
        const ret = getObject(arg0).ctrlKey;
        return ret;
    };
    imports.wbg.__wbg_documentElement_197a88c262a0aa27 = function(arg0) {
        const ret = getObject(arg0).documentElement;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_document_d249400bd7bd996d = function(arg0) {
        const ret = getObject(arg0).document;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_exec_3e2d2d0644c927df = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).exec(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_fetch_1b7e793ab8320753 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).fetch(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_firstChild_b0603462b5172539 = function(arg0) {
        const ret = getObject(arg0).firstChild;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_focus_7d08b55eba7b368d = function() { return handleError(function (arg0) {
        getObject(arg0).focus();
    }, arguments) };
    imports.wbg.__wbg_getAttribute_ea5166be2deba45e = function(arg0, arg1, arg2, arg3) {
        const ret = getObject(arg1).getAttribute(getStringFromWasm0(arg2, arg3));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_getBoundingClientRect_3e1df67095e31a84 = function(arg0) {
        const ret = getObject(arg0).getBoundingClientRect();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getBoundingClientRect_9073b0ff7574d76b = function(arg0) {
        const ret = getObject(arg0).getBoundingClientRect();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_getElementById_f827f0d6648718a8 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).getElementById(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_getItem_17f98dee3b43fa7e = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg1).getItem(getStringFromWasm0(arg2, arg3));
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_getRangeAt_4ce4631bcc2fb1ba = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).getRangeAt(arg1 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_getSelection_821202359d5703e9 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).getSelection();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_67b2ba62fc30de12 = function() { return handleError(function (arg0, arg1) {
        const ret = Reflect.get(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_get_b9b93047fe3cf45b = function(arg0, arg1) {
        const ret = getObject(arg0)[arg1 >>> 0];
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_hasAttribute_db31090c2e646f57 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).hasAttribute(getStringFromWasm0(arg1, arg2));
        return ret;
    };
    imports.wbg.__wbg_hash_dd4b49269c385c8a = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg1).hash;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    }, arguments) };
    imports.wbg.__wbg_height_592a89ec0fb63726 = function(arg0) {
        const ret = getObject(arg0).height;
        return ret;
    };
    imports.wbg.__wbg_history_b8221edd09c17656 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).history;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_id_c65402eae48fb242 = function(arg0, arg1) {
        const ret = getObject(arg1).id;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_innerHeight_05f4225d754a7929 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).innerHeight;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_innerWidth_7e0498dbd876d498 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).innerWidth;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_insertBefore_c181fb91844cd959 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).insertBefore(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_instanceof_Element_0af65443936d5154 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Element;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlDetailsElement_fcb0ca4163e13643 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof HTMLDetailsElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlInputElement_12d71bf2d15dd19e = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof HTMLInputElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlSelectElement_2ae72edd07bf938c = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof HTMLSelectElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_HtmlTextAreaElement_83a92f8ba4fb63ae = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof HTMLTextAreaElement;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Node_fbc6b87f5ed2e230 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Node;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Response_f2cc20d9f7dfd644 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Response;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_instanceof_Window_def73ea0955fc569 = function(arg0) {
        let result;
        try {
            result = getObject(arg0) instanceof Window;
        } catch (_) {
            result = false;
        }
        const ret = result;
        return ret;
    };
    imports.wbg.__wbg_isArray_a1eab7e0d067391b = function(arg0) {
        const ret = Array.isArray(getObject(arg0));
        return ret;
    };
    imports.wbg.__wbg_isCollapsed_6ed2f2b94b285dd1 = function(arg0) {
        const ret = getObject(arg0).isCollapsed;
        return ret;
    };
    imports.wbg.__wbg_isConnected_61ed8d7b311467d5 = function(arg0) {
        const ret = getObject(arg0).isConnected;
        return ret;
    };
    imports.wbg.__wbg_isContentEditable_476a31af2f281cb6 = function(arg0) {
        const ret = getObject(arg0).isContentEditable;
        return ret;
    };
    imports.wbg.__wbg_isSameNode_2a012c6251fe238d = function(arg0, arg1) {
        const ret = getObject(arg0).isSameNode(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_item_8be407c958853a13 = function(arg0, arg1) {
        const ret = getObject(arg0).item(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_item_cfe204c1386c8374 = function(arg0, arg1) {
        const ret = getObject(arg0).item(arg1 >>> 0);
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_key_7b5c6cb539be8e13 = function(arg0, arg1) {
        const ret = getObject(arg1).key;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_left_e46801720267b66d = function(arg0) {
        const ret = getObject(arg0).left;
        return ret;
    };
    imports.wbg.__wbg_length_1289a9c057721bc7 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_length_49b2ba67f0897e97 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_length_e2d2a49132c1b256 = function(arg0) {
        const ret = getObject(arg0).length;
        return ret;
    };
    imports.wbg.__wbg_localStorage_1406c99c39728187 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).localStorage;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_localeCompare_aba25b49b5adb9cb = function(arg0, arg1, arg2, arg3, arg4) {
        const ret = getObject(arg0).localeCompare(getStringFromWasm0(arg1, arg2), getObject(arg3), getObject(arg4));
        return ret;
    };
    imports.wbg.__wbg_location_350d99456c2f3693 = function(arg0) {
        const ret = getObject(arg0).location;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_matchMedia_bf8807a841d930c1 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).matchMedia(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_matches_33a2000d7f67f964 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).matches(getStringFromWasm0(arg1, arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_matches_e9ca73fbf8a3a104 = function(arg0) {
        const ret = getObject(arg0).matches;
        return ret;
    };
    imports.wbg.__wbg_metaKey_e1dd47d709a80ce5 = function(arg0) {
        const ret = getObject(arg0).metaKey;
        return ret;
    };
    imports.wbg.__wbg_navigator_1577371c070c8947 = function(arg0) {
        const ret = getObject(arg0).navigator;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_23a2665fac83c611 = function(arg0, arg1) {
        try {
            var state0 = {a: arg0, b: arg1};
            var cb0 = (arg0, arg1) => {
                const a = state0.a;
                state0.a = 0;
                try {
                    return __wbg_adapter_397(a, state0.b, arg0, arg1);
                } finally {
                    state0.a = a;
                }
            };
            const ret = new Promise(cb0);
            return addHeapObject(ret);
        } finally {
            state0.a = state0.b = 0;
        }
    };
    imports.wbg.__wbg_new_31a97dac4f10fab7 = function(arg0) {
        const ret = new Date(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_405e22f390576ce2 = function() {
        const ret = new Object();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_5f34cc0c99fcc488 = function() { return handleError(function (arg0) {
        const ret = new ResizeObserver(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_new_63847613cde5d4bc = function(arg0, arg1, arg2, arg3) {
        const ret = new RegExp(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_78feb108b6472713 = function() {
        const ret = new Array();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_99d41243f28e26b2 = function(arg0) {
        const ret = new Number(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_new_e666ce5eb84f5bb2 = function() { return handleError(function (arg0) {
        const ret = new MutationObserver(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_newnoargs_105ed471475aaf50 = function(arg0, arg1) {
        const ret = new Function(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_nextElementSibling_8472709bec4de113 = function(arg0) {
        const ret = getObject(arg0).nextElementSibling;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_nextNode_25ba1415b9dee2d2 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).nextNode();
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_nodeType_5e1153141daac26a = function(arg0) {
        const ret = getObject(arg0).nodeType;
        return ret;
    };
    imports.wbg.__wbg_nodeValue_6ff4f14870c43bd9 = function(arg0, arg1) {
        const ret = getObject(arg1).nodeValue;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_normalize_96d8e4ea8bdfb68f = function(arg0) {
        getObject(arg0).normalize();
    };
    imports.wbg.__wbg_now_807e54c39636c349 = function() {
        const ret = Date.now();
        return ret;
    };
    imports.wbg.__wbg_now_d18023d54d4e5500 = function(arg0) {
        const ret = getObject(arg0).now();
        return ret;
    };
    imports.wbg.__wbg_observe_aaa5f1933515e766 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).observe(getObject(arg1), getObject(arg2));
    }, arguments) };
    imports.wbg.__wbg_observe_d2e7378f15f7ca72 = function(arg0, arg1) {
        getObject(arg0).observe(getObject(arg1));
    };
    imports.wbg.__wbg_of_66b3ee656cbd962b = function(arg0, arg1) {
        const ret = Array.of(getObject(arg0), getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_offsetHeight_4b2bc94377e10979 = function(arg0) {
        const ret = getObject(arg0).offsetHeight;
        return ret;
    };
    imports.wbg.__wbg_offsetWidth_3cf4cc9df4051078 = function(arg0) {
        const ret = getObject(arg0).offsetWidth;
        return ret;
    };
    imports.wbg.__wbg_ok_3aaf32d069979723 = function(arg0) {
        const ret = getObject(arg0).ok;
        return ret;
    };
    imports.wbg.__wbg_parentElement_be28a1a931f9c9b7 = function(arg0) {
        const ret = getObject(arg0).parentElement;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_parentNode_9de97a0e7973ea4e = function(arg0) {
        const ret = getObject(arg0).parentNode;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_parseFloat_7e07eec2726e65d0 = function(arg0, arg1) {
        const ret = Number.parseFloat(getStringFromWasm0(arg0, arg1));
        return ret;
    };
    imports.wbg.__wbg_parseInt_01d4a03838fa55bc = function(arg0, arg1, arg2) {
        const ret = Number.parseInt(getStringFromWasm0(arg0, arg1), arg2);
        return ret;
    };
    imports.wbg.__wbg_parse_def2e24ef1252aff = function() { return handleError(function (arg0, arg1) {
        const ret = JSON.parse(getStringFromWasm0(arg0, arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_performance_c185c0cdc2766575 = function(arg0) {
        const ret = getObject(arg0).performance;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_pow_35f95838ecd24e51 = function(arg0, arg1) {
        const ret = Math.pow(arg0, arg1);
        return ret;
    };
    imports.wbg.__wbg_preventDefault_c2314fd813c02b3c = function(arg0) {
        getObject(arg0).preventDefault();
    };
    imports.wbg.__wbg_push_737cfc8c1432c2c6 = function(arg0, arg1) {
        const ret = getObject(arg0).push(getObject(arg1));
        return ret;
    };
    imports.wbg.__wbg_querySelectorAll_8ea0e513ba30ebd0 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).querySelectorAll(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_querySelector_d638ba83a95cf66a = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).querySelector(getStringFromWasm0(arg1, arg2));
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_queueMicrotask_97d92b4fcc8a61c5 = function(arg0) {
        queueMicrotask(getObject(arg0));
    };
    imports.wbg.__wbg_queueMicrotask_d3219def82552485 = function(arg0) {
        const ret = getObject(arg0).queueMicrotask;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_random_3ad904d98382defe = function() {
        const ret = Math.random();
        return ret;
    };
    imports.wbg.__wbg_rangeCount_7fd35407bb7e89cd = function(arg0) {
        const ret = getObject(arg0).rangeCount;
        return ret;
    };
    imports.wbg.__wbg_removeAllRanges_8e943cff8e7ac1c5 = function() { return handleError(function (arg0) {
        getObject(arg0).removeAllRanges();
    }, arguments) };
    imports.wbg.__wbg_removeAttribute_e419cd6726b4c62f = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).removeAttribute(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_removeChild_841bf1dc802c0a2c = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).removeChild(getObject(arg1));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_remove_282d941ca37d0c63 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).remove(getStringFromWasm0(arg1, arg2));
    }, arguments) };
    imports.wbg.__wbg_replaceState_79f3b896be12c919 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
        getObject(arg0).replaceState(getObject(arg1), getStringFromWasm0(arg2, arg3), arg4 === 0 ? undefined : getStringFromWasm0(arg4, arg5));
    }, arguments) };
    imports.wbg.__wbg_replaceWith_6ea9ccec9057b3b7 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).replaceWith(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_replaceWith_9ce9927e3141d0f6 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).replaceWith(getObject(arg1));
    }, arguments) };
    imports.wbg.__wbg_replace_5f8f0dfb7c0efb12 = function(arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).replace(getObject(arg1), getStringFromWasm0(arg2, arg3));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_requestAnimationFrame_d7fd890aaefc3246 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).requestAnimationFrame(getObject(arg1));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_resolve_4851785c9c5f573d = function(arg0) {
        const ret = Promise.resolve(getObject(arg0));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_round_d3c6f5f0c2d66c40 = function(arg0) {
        const ret = Math.round(arg0);
        return ret;
    };
    imports.wbg.__wbg_scrollHeight_6c5a5fc7dbce3f36 = function(arg0) {
        const ret = getObject(arg0).scrollHeight;
        return ret;
    };
    imports.wbg.__wbg_scrollIntoView_c8c0faa10ba6968a = function(arg0, arg1) {
        getObject(arg0).scrollIntoView(getObject(arg1));
    };
    imports.wbg.__wbg_scrollTo_26cd993048111460 = function(arg0, arg1, arg2) {
        getObject(arg0).scrollTo(arg1, arg2);
    };
    imports.wbg.__wbg_scrollTo_8163d49be09d5ebf = function(arg0, arg1) {
        getObject(arg0).scrollTo(getObject(arg1));
    };
    imports.wbg.__wbg_scrollTop_fa6a185d62b9a4ab = function(arg0) {
        const ret = getObject(arg0).scrollTop;
        return ret;
    };
    imports.wbg.__wbg_scrollX_02fa7e84cc48b8fe = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).scrollX;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_scrollY_94bf061418186bb3 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).scrollY;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_select_a6f799366a79c31b = function(arg0) {
        getObject(arg0).select();
    };
    imports.wbg.__wbg_setAttribute_2704501201f15687 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setAttribute(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setEnd_59cf2d4e38690841 = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).setEnd(getObject(arg1), arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_setInterval_cfbb32b46c873db2 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).setInterval(getObject(arg1), arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setItem_212ecc915942ab0a = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setItem(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setProperty_f2cf326652b9a713 = function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
        getObject(arg0).setProperty(getStringFromWasm0(arg1, arg2), getStringFromWasm0(arg3, arg4));
    }, arguments) };
    imports.wbg.__wbg_setStart_358e32b66ecd7f2c = function() { return handleError(function (arg0, arg1, arg2) {
        getObject(arg0).setStart(getObject(arg1), arg2 >>> 0);
    }, arguments) };
    imports.wbg.__wbg_setTimeout_f2fe5af8e3debeb3 = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).setTimeout(getObject(arg1), arg2);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_setbehavior_19b49794a6816a43 = function(arg0, arg1) {
        getObject(arg0).behavior = __wbindgen_enum_ScrollBehavior[arg1];
    };
    imports.wbg.__wbg_setbehavior_7927939a0bc6d379 = function(arg0, arg1) {
        getObject(arg0).behavior = __wbindgen_enum_ScrollBehavior[arg1];
    };
    imports.wbg.__wbg_setblock_63cb5ef6332c2d9f = function(arg0, arg1) {
        getObject(arg0).block = __wbindgen_enum_ScrollLogicalPosition[arg1];
    };
    imports.wbg.__wbg_setchildlist_c34a10b5a4d59345 = function(arg0, arg1) {
        getObject(arg0).childList = arg1 !== 0;
    };
    imports.wbg.__wbg_setclassName_6bdd0705e646d028 = function(arg0, arg1, arg2) {
        getObject(arg0).className = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_sethidden_c9dddd5ae4b3314f = function(arg0, arg1) {
        getObject(arg0).hidden = arg1 !== 0;
    };
    imports.wbg.__wbg_setid_d1300d55a412791b = function(arg0, arg1, arg2) {
        getObject(arg0).id = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setinnerHTML_31bde41f835786f7 = function(arg0, arg1, arg2) {
        getObject(arg0).innerHTML = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setlastindex_b7f88ee3f88ca390 = function(arg0, arg1) {
        getObject(arg0).lastIndex = arg1 >>> 0;
    };
    imports.wbg.__wbg_setnodeValue_58cb1b2f6b6c33d2 = function(arg0, arg1, arg2) {
        getObject(arg0).nodeValue = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setopen_603cdb614114bda0 = function(arg0, arg1) {
        getObject(arg0).open = arg1 !== 0;
    };
    imports.wbg.__wbg_setpassive_57a5a4c4b00a7c62 = function(arg0, arg1) {
        getObject(arg0).passive = arg1 !== 0;
    };
    imports.wbg.__wbg_setscrollRestoration_3ac82da5fd0922ae = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).scrollRestoration = __wbindgen_enum_ScrollRestoration[arg1];
    }, arguments) };
    imports.wbg.__wbg_setscrollTop_10fcb58772498965 = function(arg0, arg1) {
        getObject(arg0).scrollTop = arg1;
    };
    imports.wbg.__wbg_setselectionEnd_d0dfa434c176031a = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).selectionEnd = arg1 === 0x100000001 ? undefined : arg1;
    }, arguments) };
    imports.wbg.__wbg_setselectionStart_253f7c8500199419 = function() { return handleError(function (arg0, arg1) {
        getObject(arg0).selectionStart = arg1 === 0x100000001 ? undefined : arg1;
    }, arguments) };
    imports.wbg.__wbg_setsubtree_a07feb5885be0897 = function(arg0, arg1) {
        getObject(arg0).subtree = arg1 !== 0;
    };
    imports.wbg.__wbg_settextContent_d29397f7b994d314 = function(arg0, arg1, arg2) {
        getObject(arg0).textContent = arg1 === 0 ? undefined : getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_settop_49206b36e1f49d3d = function(arg0, arg1) {
        getObject(arg0).top = arg1;
    };
    imports.wbg.__wbg_setvalue_08d17a42e5d5069d = function(arg0, arg1, arg2) {
        getObject(arg0).value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setvalue_6ad9ef6c692ea746 = function(arg0, arg1, arg2) {
        getObject(arg0).value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_setvalue_f76614f551612e39 = function(arg0, arg1, arg2) {
        getObject(arg0).value = getStringFromWasm0(arg1, arg2);
    };
    imports.wbg.__wbg_showModal_922f39328729f17e = function() { return handleError(function (arg0) {
        getObject(arg0).showModal();
    }, arguments) };
    imports.wbg.__wbg_splitText_2cb7c09323baf1c3 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).splitText(arg1 >>> 0);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_startContainer_9d98d798f1cb1b68 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).startContainer;
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_startOffset_10d6ef767bb32c89 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).startOffset;
        return ret;
    }, arguments) };
    imports.wbg.__wbg_static_accessor_GLOBAL_88a902d13a557d07 = function() {
        const ret = typeof global === 'undefined' ? null : global;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_GLOBAL_THIS_56578be7e9f832b0 = function() {
        const ret = typeof globalThis === 'undefined' ? null : globalThis;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_SELF_37c5d418e4bf5819 = function() {
        const ret = typeof self === 'undefined' ? null : self;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_static_accessor_WINDOW_5de37043a91a9c40 = function() {
        const ret = typeof window === 'undefined' ? null : window;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_status_f6360336ca686bf0 = function(arg0) {
        const ret = getObject(arg0).status;
        return ret;
    };
    imports.wbg.__wbg_stringify_f7ed6987935b4a24 = function() { return handleError(function (arg0) {
        const ret = JSON.stringify(getObject(arg0));
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_style_fb30c14e5815805c = function(arg0) {
        const ret = getObject(arg0).style;
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_tagName_b284ab9c1479c38d = function(arg0, arg1) {
        const ret = getObject(arg1).tagName;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_takeRecords_73e29c942adb5e82 = function(arg0) {
        const ret = getObject(arg0).takeRecords();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_target_0a62d9d79a2a1ede = function(arg0) {
        const ret = getObject(arg0).target;
        return isLikeNone(ret) ? 0 : addHeapObject(ret);
    };
    imports.wbg.__wbg_test_7f0ac7b9d67b7a48 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).test(getStringFromWasm0(arg1, arg2));
        return ret;
    };
    imports.wbg.__wbg_textContent_215d0f87d539368a = function(arg0, arg1) {
        const ret = getObject(arg1).textContent;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_text_7805bea50de2af49 = function() { return handleError(function (arg0) {
        const ret = getObject(arg0).text();
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_then_44b73946d2fb3e7d = function(arg0, arg1) {
        const ret = getObject(arg0).then(getObject(arg1));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_then_48b406749878a531 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).then(getObject(arg1), getObject(arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toFixed_43af7895cf202c17 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).toFixed(arg1);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_toString_5285597960676b7b = function(arg0) {
        const ret = getObject(arg0).toString();
        return addHeapObject(ret);
    };
    imports.wbg.__wbg_toString_ba82658ec370add0 = function() { return handleError(function (arg0, arg1) {
        const ret = getObject(arg0).toString(arg1);
        return addHeapObject(ret);
    }, arguments) };
    imports.wbg.__wbg_toggle_b764823ee3836d0d = function() { return handleError(function (arg0, arg1, arg2, arg3) {
        const ret = getObject(arg0).toggle(getStringFromWasm0(arg1, arg2), arg3 !== 0);
        return ret;
    }, arguments) };
    imports.wbg.__wbg_toggle_eb55caf936454eda = function() { return handleError(function (arg0, arg1, arg2) {
        const ret = getObject(arg0).toggle(getStringFromWasm0(arg1, arg2));
        return ret;
    }, arguments) };
    imports.wbg.__wbg_top_ec9fceb1f030f2ea = function(arg0) {
        const ret = getObject(arg0).top;
        return ret;
    };
    imports.wbg.__wbg_valueOf_fdbb54fcdfe33477 = function(arg0) {
        const ret = getObject(arg0).valueOf();
        return ret;
    };
    imports.wbg.__wbg_value_1d971aac958c6f2f = function(arg0, arg1) {
        const ret = getObject(arg1).value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_91cbf0dd3ab84c1e = function(arg0, arg1) {
        const ret = getObject(arg1).value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_value_d2c3b815cdf98d46 = function(arg0, arg1) {
        const ret = getObject(arg1).value;
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbg_visibilityState_f3cc18a6f3831137 = function(arg0) {
        const ret = getObject(arg0).visibilityState;
        return (__wbindgen_enum_VisibilityState.indexOf(ret) + 1 || 3) - 1;
    };
    imports.wbg.__wbg_width_f0759bd8bad335bd = function(arg0) {
        const ret = getObject(arg0).width;
        return ret;
    };
    imports.wbg.__wbg_writeText_51c338e8ae4b85b9 = function(arg0, arg1, arg2) {
        const ret = getObject(arg0).writeText(getStringFromWasm0(arg1, arg2));
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_cb_drop = function(arg0) {
        const obj = takeObject(arg0).original;
        if (obj.cnt-- == 1) {
            obj.a = 0;
            return true;
        }
        const ret = false;
        return ret;
    };
    imports.wbg.__wbindgen_closure_wrapper1944 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 247, __wbg_adapter_39);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper439 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 12, __wbg_adapter_30);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper440 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 12, __wbg_adapter_33);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_closure_wrapper442 = function(arg0, arg1, arg2) {
        const ret = makeMutClosure(arg0, arg1, 15, __wbg_adapter_36);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_debug_string = function(arg0, arg1) {
        const ret = debugString(getObject(arg1));
        const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        const len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_is_falsy = function(arg0) {
        const ret = !getObject(arg0);
        return ret;
    };
    imports.wbg.__wbindgen_is_function = function(arg0) {
        const ret = typeof(getObject(arg0)) === 'function';
        return ret;
    };
    imports.wbg.__wbindgen_is_null = function(arg0) {
        const ret = getObject(arg0) === null;
        return ret;
    };
    imports.wbg.__wbindgen_is_object = function(arg0) {
        const val = getObject(arg0);
        const ret = typeof(val) === 'object' && val !== null;
        return ret;
    };
    imports.wbg.__wbindgen_is_undefined = function(arg0) {
        const ret = getObject(arg0) === undefined;
        return ret;
    };
    imports.wbg.__wbindgen_number_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'number' ? obj : undefined;
        getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
    };
    imports.wbg.__wbindgen_number_new = function(arg0) {
        const ret = arg0;
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_clone_ref = function(arg0) {
        const ret = getObject(arg0);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_object_drop_ref = function(arg0) {
        takeObject(arg0);
    };
    imports.wbg.__wbindgen_string_get = function(arg0, arg1) {
        const obj = getObject(arg1);
        const ret = typeof(obj) === 'string' ? obj : undefined;
        var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_export_1, wasm.__wbindgen_export_2);
        var len1 = WASM_VECTOR_LEN;
        getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
        getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
    };
    imports.wbg.__wbindgen_string_new = function(arg0, arg1) {
        const ret = getStringFromWasm0(arg0, arg1);
        return addHeapObject(ret);
    };
    imports.wbg.__wbindgen_throw = function(arg0, arg1) {
        throw new Error(getStringFromWasm0(arg0, arg1));
    };

    return imports;
}

function __wbg_init_memory(imports, memory) {

}

function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    __wbg_init.__wbindgen_wasm_module = module;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;



    return wasm;
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (typeof module !== 'undefined') {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();

    __wbg_init_memory(imports);

    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }

    const instance = new WebAssembly.Instance(module, imports);

    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (typeof module_or_path !== 'undefined') {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (typeof module_or_path === 'undefined') {
        module_or_path = new URL('studyhub_web_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    __wbg_init_memory(imports);

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync };
export default __wbg_init;
