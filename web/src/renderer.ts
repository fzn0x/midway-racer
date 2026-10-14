/** Drawing the emulator's picture on a canvas. */

export class Renderer {
  readonly #context: CanvasRenderingContext2D;
  readonly #image: ImageData;

  /** Sizes `canvas` to the picture; CSS scales it to the screen. */
  constructor(canvas: HTMLCanvasElement, width: number, height: number) {
    canvas.width = width;
    canvas.height = height;
    const context = canvas.getContext("2d", { alpha: false });
    if (!context) {
      throw new Error("this browser cannot draw on a canvas");
    }
    this.#context = context;
    this.#image = context.createImageData(width, height);
  }

  /**
   * Draws the RGBA picture the emulator wrote at `pointer` in `memory`.
   * The view is made afresh every time because WebAssembly memory may
   * have grown, which detaches older views.
   */
  draw(memory: WebAssembly.Memory, pointer: number): void {
    const pixels = new Uint8ClampedArray(
      memory.buffer,
      pointer,
      this.#image.data.length,
    );
    this.#image.data.set(pixels);
    this.#context.putImageData(this.#image, 0, 0);
  }
}
