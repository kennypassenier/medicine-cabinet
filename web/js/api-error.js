// The one failure type of the data layer: the server's own sentence with its
// remedy, or the same words written locally when the server is not asked.

/** An API failure with the server's own sentence and its remedy. */
export class ApiError extends Error {
  /** @param {string} message @param {string} remedy @param {number} status */
  constructor(message, remedy, status) {
    super(message);
    this.name = "ApiError";
    this.remedy = remedy;
    this.status = status;
  }
}
