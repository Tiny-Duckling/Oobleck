from rest_framework import viewsets

from .models import Level, Subject, Category, Topic
from .serializers import (
    LevelSerializer,
    SubjectSerializer,
    CategorySerializer,
    TopicSerializer,
    QuestionGroupSerializer,
)
from .permissions import IsStaffOrReadOnly, IsTeacherOfSubject


class LevelViewSet(viewsets.ModelViewSet):
    queryset = Level.objects.all()
    serializer_class = LevelSerializer
    permission_classes = [IsStaffOrReadOnly]


class CategoryViewSet(viewsets.ModelViewSet):
    queryset = Category.objects.all()
    serializer_class = CategorySerializer
    permission_classes = [IsStaffOrReadOnly]


class SubjectViewSet(viewsets.ModelViewSet):
    queryset = Subject.objects.all()
    serializer_class = SubjectSerializer
    permission_classes = [IsStaffOrReadOnly]


class TopicViewSet(viewsets.ModelViewSet):
    queryset = Topic.objects.all()
    serializer_class = TopicSerializer
    permission_classes = [IsTeacherOfSubject]


class QuestionGroupViewSet(viewsets.ModelViewSet):
    queryset = Level.objects.all()
    serializer_class = QuestionGroupSerializer
    permission_classes = [IsTeacherOfSubject]
