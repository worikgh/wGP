## Calculate if input % 3 is 0, 1, or 2
q <- sample(seq(1,10000), 10000)
data <- cbind(q, q%%3)
colnames(data) <- c("Q", "Object")
write.csv(data, "data.in", row.names=FALSE, quote=FALSE)
