from django.conf import settings
from django.db import models


class AnswerSheet(models.Model):
    exam = models.ForeignKey('exams.Exam', on_delete=models.CASCADE)

    created_at = models.DateTimeField(auto_now_add=True)
    created_by = models.ForeignKey(settings.AUTH_USER_MODEL, on_delete=models.CASCADE)


class Answer(models.Model):
    answer_sheet = models.ForeignKey(AnswerSheet, on_delete=models.CASCADE)
    question = models.ForeignKey('exams.Question', on_delete=models.CASCADE)

    text = models.TextField()
    value = models.DecimalField(max_digits=30, decimal_places=8)
    choice = models.ForeignKey('exams.Choice', on_delete=models.CASCADE)
