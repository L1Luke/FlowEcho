#ifndef FlowEchoCoreFFI_h
#define FlowEchoCoreFFI_h

#ifdef __cplusplus
extern "C" {
#endif

char *flowecho_start_pairing(const char *input_ptr);
char *flowecho_pair_device(const char *input_ptr);
char *flowecho_send_text(const char *input_ptr);
char *flowecho_send_file(const char *input_ptr);
char *flowecho_resume_transfer(const char *input_ptr);
char *flowecho_latest_received_text(const char *input_ptr);
char *flowecho_latest_received_file(const char *input_ptr);
void flowecho_free_string(char *ptr);

#ifdef __cplusplus
}
#endif

#endif
