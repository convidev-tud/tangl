FROM rust:1.94.1

WORKDIR /usr/src/tangl
COPY . .

RUN make
RUN echo source /root/.local/share/bash-completion/completions/tangl >> ~/.bashrc

CMD ["bash"]