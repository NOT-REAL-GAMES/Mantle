self.onmessage = ({ data }) => { self.postMessage(data, [data.buffer]); };
